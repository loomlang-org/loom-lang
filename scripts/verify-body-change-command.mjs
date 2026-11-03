import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// Reviewed composition stays read-only until an exact token is applied. The
// main entry deliberately fails; only the explicitly compiled tests may run.
export function verifyBodyChangeCommand(loom, directory, environment, standard) {
  mkdirSync(directory);
  directory = realpathSync.native(directory);
  function invoke(executable, args, cwd = directory, env = environment) {
    const result = spawnSync(executable, args, { cwd, env, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
    if (result.error) throw result.error;
    return result;
  }
  function success(result) { assert.equal(result.status, 0, result.stdout + result.stderr); return result.stdout; }
  const change = (...args) => invoke(loom, ["change", ...args]);
  function token(result) {
    const found = success(result).match(/^review token: ([a-f0-9]{64})$/m)?.[1];
    assert.ok(found, "missing body-composition review token");
    return found;
  }
  function inventory(root) {
    return readdirSync(root, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name)).flatMap(entry => {
      const path = join(root, entry.name);
      return entry.isDirectory() ? inventory(path).map(([name, hash]) => [join(entry.name, name), hash]) :
        [[entry.name, createHash("sha256").update(readFileSync(path)).digest("hex")]];
    });
  }
  function snapshots(label, source) {
    const root = join(directory, label); mkdirSync(root);
    const paths = ["base", "left", "right"].map(name => join(root, name));
    mkdirSync(paths[0]); writeFileSync(join(paths[0], "main.loom"), source);
    success(change("init", "sample", paths[0]));
    cpSync(paths[0], paths[1], { recursive: true, filter: () => true });
    cpSync(paths[0], paths[2], { recursive: true, filter: () => true });
    return paths;
  }
  function review(paths, args = paths) {
    const before = paths.map(inventory);
    const reviewed = token(change(...args));
    assert.deepEqual(paths.map(inventory), before, "body review changed source inputs");
    return reviewed;
  }
  function runTests(root, source, levels = [2]) {
    writeFileSync(join(root, "composed_test.loom"), source);
    for (const level of levels) {
      const binary = join(directory, `tests-${level}-${counter++}${process.platform === "win32" ? ".exe" : ""}`);
      const compiled = success(invoke(loom, ["test", root, "--no-run", "--output", binary], directory,
        { ...environment, LOOM_OPT_LEVEL: String(level) }));
      assert.match(compiled, /^compiled 1 tests to /);
      assert.equal(success(invoke(binary, [], directory, { ...environment, LOOM_GC_STRESS: "1" })), "");
    }
  }
  let counter = 0;
  const source = `fn mark(log List[Int], input Int) Int {
    log[0] = log[0] * 10 + input
    input
}
fn calculate(log List[Int]) Int { mark(log, 1) + mark(log, 2) }
fn main() { assert false }
`;
  const paths = snapshots("effectful 雪", source);
  const left = source.replace("mark(log, 1)", "mark(log, 3)");
  const right = source.replace("mark(log, 2)", "mark(log, 4)");
  writeFileSync(join(paths[1], "main.loom"), left); writeFileSync(join(paths[2], "main.loom"), right);
  const reviewed = review(paths), rejected = join(directory, "stale");
  writeFileSync(join(paths[2], "main.loom"), right + "\n// changed after review\n");
  assert.equal(change("--apply", reviewed, rejected, ...paths).status, 1);
  assert.ok(!existsSync(rejected)); writeFileSync(join(paths[2], "main.loom"), right);
  const merged = join(directory, "merged"); success(change("--apply", reviewed, merged, ...paths));
  assert.match(readFileSync(join(merged, "main.loom"), "utf8"), /mark\(log, 3\) \+ mark\(log, 4\)/);
  assert.equal(readFileSync(join(merged, ".loom-ids"), "utf8"), readFileSync(join(paths[0], ".loom-ids"), "utf8"));
  const tests = "test fn composed_calls_keep_effect_order() { let log = [0]\n    assert calculate(log) == 7\n    assert log[0] == 34 }\n";
  runTests(merged, tests, [0, 2]);
  writeFileSync(join(paths[2], "main.loom"), source.replace("mark(log, 1)", "mark(log, 5)"));
  const conflict = paths.map(inventory);
  assert.equal(change(...paths).status, 1, "overlapping operand edits were merged");
  assert.deepEqual(paths.map(inventory), conflict); writeFileSync(join(paths[2], "main.loom"), right);

  const contractSource = "fn value() Int ensures result <= 5 { 1 + 1 }\nfn main() { assert false }\n";
  const contracts = snapshots("contracts", contractSource);
  writeFileSync(join(contracts[1], "main.loom"), contractSource.replace("1 + 1", "4 + 1"));
  writeFileSync(join(contracts[2], "main.loom"), contractSource.replace("1 + 1", "1 + 4"));
  for (const branch of contracts) success(invoke(loom, ["check", branch]));
  const contractInputs = contracts.map(inventory), contractRejected = change(...contracts);
  assert.equal(contractRejected.status, 1, "composition bypassed a required function contract");
  assert.match(contractRejected.stdout + contractRejected.stderr, /proof|postcondition/);
  assert.deepEqual(contracts.map(inventory), contractInputs);

  const reorderSource = "fn a(value Int) Int { value }\nfn b(value Int) Int { value * 10 }\n" +
    "fn calculate() Int { a(1) + b(1) }\nfn main() { assert false }\n";
  const reordered = snapshots("reordered calls", reorderSource);
  writeFileSync(join(reordered[1], "main.loom"), reorderSource.replace("a(1) + b(1)", "b(1) + a(1)"));
  writeFileSync(join(reordered[2], "main.loom"), reorderSource.replace("a(1) + b(1)", "a(2) + b(1)"));
  for (const branch of reordered) success(invoke(loom, ["check", branch]));
  const reorderedInputs = reordered.map(inventory);
  assert.equal(change(...reordered).status, 1, "argument edit was assigned to a reordered call");
  assert.deepEqual(reordered.map(inventory), reorderedInputs);

  const repository = join(directory, "repository"); mkdirSync(repository);
  const git = (...args) => success(invoke("git", args, repository)).trim();
  git("init", "--template=");
  function commit(snapshot, parent) {
    for (const entry of readdirSync(repository)) if (entry !== ".git") rmSync(join(repository, entry), { recursive: true, force: true });
    for (const entry of readdirSync(snapshot)) cpSync(join(snapshot, entry), join(repository, entry), { recursive: true, filter: () => true });
    // These snapshots can differ only in same-sized numeric literals. Rebuild
    // the disposable index so coarse filesystem timestamps cannot hide edits.
    git("read-tree", "--empty");
    git("-c", "core.autocrlf=false", "add", "--all");
    const tree = git("write-tree");
    const revision = git("-c", "user.name=Loom test", "-c", "user.email=loom@example.invalid", "-c", "commit.gpgsign=false",
      "commit-tree", tree, "-m", "body composition fixture", ...(parent ? ["-p", parent] : []));
    for (const [name] of inventory(snapshot)) {
      const blob = success(invoke("git", ["cat-file", "blob", `${revision}:${name.replaceAll("\\", "/")}`], repository));
      assert.equal(blob, readFileSync(join(snapshot, name), "utf8"), "Git fixture did not capture the exact snapshot");
    }
    return revision;
  }
  const base = commit(paths[0]), a = commit(paths[1], base), b = commit(paths[2], base); git("update-ref", "HEAD", b);
  const gitArgs = ["--git", "git", repository, ".", base, a, "HEAD"], status = git("status", "--porcelain=v1");
  const index = readFileSync(join(repository, ".git", "index"));
  const gitOutput = join(directory, "git-merged"); success(change("--apply", token(change(...gitArgs)), gitOutput, ...gitArgs));
  assert.equal(readFileSync(join(gitOutput, "main.loom"), "utf8"), readFileSync(join(merged, "main.loom"), "utf8"));
  assert.equal(git("status", "--porcelain=v1"), status); assert.deepEqual(readFileSync(join(repository, ".git", "index")), index);
  assert.equal(git("rev-parse", "HEAD"), b); runTests(gitOutput, tests);

  const generic = `concept Read { fn read(self Self) Int }
impl Read for Int { fn read(self Int) Int { self + 10 + 20 } }
impl Read for Bool { fn read(self Bool) Int { if self { 1 } else { 0 } } }
fn generic_read[T Read](value T) Int { value.read() }
fn entry() Int { generic_read(7) + generic_read(true) }
fn main() { assert false }
`;
  const methods = snapshots("generic", generic);
  writeFileSync(join(methods[1], "main.loom"), generic.replace("self + 10 + 20", "self + 11 + 20"));
  writeFileSync(join(methods[2], "main.loom"), generic.replace("self + 10 + 20", "self + 10 + 22"));
  const methodOutput = join(directory, "generic-merged"); success(change("--apply", review(methods), methodOutput, ...methods));
  runTests(methodOutput, "test fn both_observed_methods_keep_their_identity() { assert entry() == 41 }\n");

  if (standard) {
    const module = join(directory, "module-base"); mkdirSync(join(module, "provider"), { recursive: true });
    writeFileSync(join(module, "loom.toml"), '[module]\nname="sample"\n');
    const provider = source.slice(0, source.indexOf("fn main")).replace("fn calculate", "pub fn calculate");
    writeFileSync(join(module, "provider", "api.loom"), provider);
    writeFileSync(join(module, "main.loom"), "import sample.provider.calculate\nfn main() { assert false }\n");
    success(change("init", "sample", module)); success(change("init", "sample.provider", join(module, "provider")));
    const moduleLeft = join(directory, "module-left"), moduleRight = join(directory, "module-right");
    cpSync(module, moduleLeft, { recursive: true, filter: () => true }); cpSync(module, moduleRight, { recursive: true, filter: () => true });
    writeFileSync(join(moduleLeft, "provider", "api.loom"), provider.replace("mark(log, 1)", "mark(log, 3)"));
    writeFileSync(join(moduleRight, "provider", "api.loom"), provider.replace("mark(log, 2)", "mark(log, 4)"));
    const moduleArgs = ["--module", standard, module, moduleLeft, moduleRight];
    const moduleOutput = join(directory, "module-merged");
    success(change("--apply", review([module, moduleLeft, moduleRight], moduleArgs), moduleOutput, ...moduleArgs));
    runTests(moduleOutput, tests);

    const moduleBaseCommit = commit(module), moduleLeftCommit = commit(moduleLeft, moduleBaseCommit);
    const moduleRightCommit = commit(moduleRight, moduleBaseCommit); git("update-ref", "HEAD", moduleRightCommit);
    const moduleGitArgs = ["--module", module, standard, "--git", "git", repository, ".",
      moduleBaseCommit, moduleLeftCommit, "HEAD"];
    const moduleStatus = git("status", "--porcelain=v1"), moduleIndex = readFileSync(join(repository, ".git", "index"));
    const moduleGitOutput = join(directory, "module-git-merged");
    success(change("--apply", token(change(...moduleGitArgs)), moduleGitOutput, ...moduleGitArgs));
    assert.equal(git("status", "--porcelain=v1"), moduleStatus);
    assert.deepEqual(readFileSync(join(repository, ".git", "index")), moduleIndex);
    assert.equal(git("rev-parse", "HEAD"), moduleRightCommit); runTests(moduleGitOutput, tests);
  }
  return { reviewed, snapshots: paths, merged, gitOutput, methodOutput };
}
