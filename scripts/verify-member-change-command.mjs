import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// Delivered CLI workflow: explicit intent, a persisted draft, checked method
// rename + file move + independent edit, stale review, directory and Git apply.
export function verifyMemberChangeCommand(loom, directory, environment, standard) {
  mkdirSync(directory);
  function command(executable, args, cwd = directory) {
    const result = spawnSync(executable, args, {
      cwd, env: environment, encoding: "utf8", maxBuffer: 4 * 1024 * 1024,
    });
    if (result.error) throw result.error;
    return result;
  }
  const change = (...args) => command(loom, ["change", ...args]);
  function success(result) {
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
  }
  function token(result) {
    const value = success(result).match(/^review token: ([a-f0-9]{64})$/m)?.[1];
    assert.ok(value, "missing method review token");
    return value;
  }
  const base = join(directory, "base");
  const left = join(directory, "left");
  const right = join(directory, "right");
  mkdirSync(base);
  const source = `concept Read {
    fn read(self Self) Int
}
impl Read for Int {
    fn read(self Int) Int { self + 1 }
}
fn entry() Int {
    let value = 7
    value.read()
}
`;
  writeFileSync(join(base, "main.loom"), source);
  success(change("init", "sample", base));
  const sidecar = readFileSync(join(base, ".loom-ids"), "utf8");
  const identities = sidecar.split("\n").filter((row) => row.startsWith("id ")).map((row) => row.split(" "));
  const roots = identities.filter((row) => row.length === 6);
  const methods = identities.filter((row) => row.length === 7);
  assert.equal(roots.length, 3);
  assert.equal(methods.length, 2);
  assert.match(success(change("scan", base)), /owner=d-/);
  cpSync(base, left, { recursive: true, filter: () => true });
  cpSync(base, right, { recursive: true, filter: () => true });
  rmSync(join(left, "main.loom"));
  writeFileSync(join(left, "moved.loom"), source.replaceAll("read(", "inspect("));
  assert.equal(change("refresh", left).status, 1, "ambiguous move was guessed");
  assert.equal(readFileSync(join(left, ".loom-ids"), "utf8"), sidecar);
  success(change("refresh", left, "--draft"));
  assert.match(readFileSync(join(left, ".loom-ids"), "utf8"), /^pending /m);
  assert.equal(change(base, left, right).status, 1, "pending identities were merged");
  const choices = roots.flatMap(([, id, , name, , key]) => ["--move", id, "moved.loom", name, key]);
  choices.push(...methods.flatMap(([, id, , , , key, owner]) => ["--move-member", id, owner, "moved.loom", "inspect", key]));
  success(change("refresh", left, ...choices));
  writeFileSync(join(right, "main.loom"), source.replace("self + 1", "self + 2"));
  success(change("refresh", right));
  const reviewed = token(change(base, left, right));
  const rejected = join(directory, "rejected");
  writeFileSync(join(right, "main.loom"), `${source.replace("self + 1", "self + 2")}\n// changed after review\n`);
  assert.equal(change("--apply", reviewed, rejected, base, left, right).status, 1);
  assert.ok(!existsSync(rejected));
  writeFileSync(join(right, "main.loom"), source.replace("self + 1", "self + 2"));
  const output = join(directory, "merged");
  success(change("--apply", reviewed, output, base, left, right));
  const merged = readFileSync(join(output, "moved.loom"), "utf8");
  assert.match(merged, /fn inspect\(self Self\)/);
  assert.match(merged, /value\.inspect\(\)/);
  assert.match(merged, /self \+ 2/);
  const retained = readFileSync(join(output, ".loom-ids"), "utf8");
  for (const [, id, , , , key, owner] of methods) {
    assert.ok(retained.includes(`id ${id} fn inspect moved.loom ${key} ${owner}\n`));
  }
  assert.equal(readFileSync(join(base, ".loom-ids"), "utf8"), sidecar);
  writeFileSync(join(output, "merged_test.loom"), "test fn renamed_and_edited_method() { assert entry() == 9 }\n");
  assert.equal(success(command(loom, ["test", output])), "1 tests passed\n");

  const repository = join(directory, "repository");
  mkdirSync(repository);
  const git = (...args) => success(command("git", args, repository)).trim();
  git("init", "--template=");
  function commit(snapshot, parent) {
    for (const name of ["main.loom", "moved.loom", ".loom-ids"]) {
      rmSync(join(repository, name), { force: true });
      if (existsSync(join(snapshot, name))) cpSync(join(snapshot, name), join(repository, name));
    }
    git("-c", "core.autocrlf=false", "add", "--all");
    const tree = git("write-tree");
    return git("-c", "user.name=Loom test", "-c", "user.email=loom@example.invalid", "-c", "commit.gpgsign=false",
      "commit-tree", tree, "-m", "owned method fixture", ...(parent ? ["-p", parent] : []));
  }
  const baseline = commit(base);
  const renamed = commit(left, baseline);
  const edited = commit(right, baseline);
  git("update-ref", "HEAD", edited);
  const status = git("status", "--porcelain=v1");
  const index = readFileSync(join(repository, ".git", "index"));
  const args = ["--git", "git", repository, ".", baseline, renamed, "HEAD"];
  const gitToken = token(change(...args));
  const moved = git("-c", "user.name=Loom test", "-c", "user.email=loom@example.invalid", "-c", "commit.gpgsign=false",
    "commit-tree", git("write-tree"), "-p", edited, "-m", "same bytes, new basis");
  git("update-ref", "HEAD", moved);
  const gitRejected = join(directory, "git-rejected");
  assert.equal(change("--apply", gitToken, gitRejected, ...args).status, 1);
  assert.ok(!existsSync(gitRejected));
  git("update-ref", "HEAD", edited);
  const gitOutput = join(directory, "git-merged");
  success(change("--apply", gitToken, gitOutput, ...args));
  assert.equal(readFileSync(join(gitOutput, "moved.loom"), "utf8"), merged);
  assert.equal(readFileSync(join(gitOutput, ".loom-ids"), "utf8"), retained);
  assert.equal(git("rev-parse", "HEAD"), edited);
  assert.deepEqual(readFileSync(join(repository, ".git", "index")), index);
  assert.equal(git("status", "--porcelain=v1"), status);

  if (standard) {
    const moduleBase = join(directory, "module-base");
    const moduleLeft = join(directory, "module-left");
    const moduleRight = join(directory, "module-right");
    mkdirSync(join(moduleBase, "lib"), { recursive: true });
    writeFileSync(join(moduleBase, "loom.toml"), '[module]\nname = "sample"\n');
    const publicLibrary = source.slice(0, source.indexOf("fn entry")).replace("concept Read", "pub concept Read");
    const consumer = `import sample.lib.Read
fn entry() Int { let value = 7
    value.read() }
fn dynamic_read(value dyn Read) Int { value.read() }
fn generic_read[T Read](value T) Int { value.read() }
`;
    const tests = `test fn public_method_available() {
    assert entry() > 0 && dynamic_read(7) > 0
}
`;
    writeFileSync(join(moduleBase, "lib", "api.loom"), publicLibrary);
    writeFileSync(join(moduleBase, "main.loom"), consumer);
    writeFileSync(join(moduleBase, "main_test.loom"), tests);
    success(change("init", "sample", moduleBase));
    success(change("init", "sample.lib", join(moduleBase, "lib")));
    const publicMethods = readFileSync(join(moduleBase, "lib", ".loom-ids"), "utf8").split("\n")
      .map((row) => row.split(" ")).filter((row) => row[0] === "id" && row.length === 7);
    cpSync(moduleBase, moduleLeft, { recursive: true, filter: () => true });
    cpSync(moduleBase, moduleRight, { recursive: true, filter: () => true });
    writeFileSync(join(moduleLeft, "lib", "api.loom"), publicLibrary.replaceAll("read(", "inspect("));
    writeFileSync(join(moduleLeft, "main.loom"), consumer.replaceAll(".read(", ".inspect("));
    success(change("refresh", moduleLeft));
    success(change("refresh", join(moduleLeft, "lib"), ...publicMethods.flatMap(([, id, , , , key, owner]) =>
      ["--move-member", id, owner, "api.loom", "inspect", key])));
    writeFileSync(join(moduleRight, "lib", "api.loom"), publicLibrary.replace("self + 1", "self + 2"));
    success(change("refresh", join(moduleRight, "lib")));
    const moduleArgs = ["--module", standard, moduleBase, moduleLeft, moduleRight];
    const moduleToken = token(change(...moduleArgs));
    const moduleOutput = join(directory, "module-merged");
    success(change("--apply", moduleToken, moduleOutput, ...moduleArgs));
    assert.match(readFileSync(join(moduleOutput, "main.loom"), "utf8"), /value\.inspect\(\)/);
    assert.match(readFileSync(join(moduleOutput, "lib", "api.loom"), "utf8"), /self \+ 2/);
    assert.equal(success(command(loom, ["test", moduleOutput])), "1 tests passed\n");
  }
}
