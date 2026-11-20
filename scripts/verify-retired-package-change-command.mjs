import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export function verifyRetiredPackageChangeCommand(loom, directory, environment, standard) {
  mkdirSync(directory);
  const base = join(directory, "base"), left = join(directory, "left"), right = join(directory, "right");
  mkdirSync(base);
  writeFileSync(join(base, "loom.toml"), '[module]\nname = "sample"\n');
  // Review must never run the application entry.
  writeFileSync(join(base, "main.loom"), "fn main() { assert false }\nfn amount() Int { 1 }\n");
  const history = join(base, "history"); mkdirSync(history);
  writeFileSync(join(history, "value.loom"), "fn source() Int { 1 }\n");
  function invoke(executable, args, env = environment) {
    const outcome = spawnSync(executable, args, { cwd: directory, env, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
    if (outcome.error) throw outcome.error;
    return outcome;
  }
  function success(outcome) { assert.equal(outcome.status, 0, outcome.stdout + outcome.stderr); return outcome.stdout; }
  const change = (...args) => invoke(loom, ["change", ...args]);
  const review = (...paths) => change("--module", standard, ...paths);
  const metadataAt = root => join(root, "history", ".loom-ids");
  success(change("init", "sample", base));
  success(change("init", "sample.history", history));
  const parent = readFileSync(metadataAt(base), "utf8").match(/^id (\S+) fn source /m)?.[1];
  assert.ok(parent);
  writeFileSync(join(history, "value.loom"), "fn source() Int { 1 }\nfn copied() Int { 1 }\n");
  success(change("refresh", history, "--derive", parent, "value.loom", "copied", "-"));
  const child = readFileSync(metadataAt(base), "utf8").match(/^id (\S+) fn copied /m)?.[1];
  assert.ok(child && child !== parent);
  cpSync(base, left, { recursive: true, filter: () => true });
  cpSync(base, right, { recursive: true, filter: () => true });
  rmSync(join(left, "history", "value.loom"));
  success(change("refresh", join(left, "history"), "--delete", parent, "--delete", child));
  const retired = readFileSync(metadataAt(left), "utf8");
  assert.ok(!/^id /m.test(retired));
  assert.match(retired, new RegExp(`^origin ${child} ${parent}$`, "m"));
  writeFileSync(join(right, "main.loom"), "fn main() { assert false }\nfn amount() Int { 2 }\n");
  const preview = success(review(base, left, right));
  assert.match(preview, /^removed package history$/m);
  assert.match(preview, /^retained package history history$/m);
  const token = preview.match(/^review token: ([a-f0-9]{64})$/m)?.[1]; assert.ok(token);
  const rejected = join(directory, "rejected");
  writeFileSync(metadataAt(left), retired + "# new review basis\n");
  assert.equal(change("--apply", token, rejected, "--module", standard, base, left, right).status, 1);
  assert.ok(!existsSync(rejected));
  writeFileSync(metadataAt(left), retired);
  const output = join(directory, "output");
  success(change("--apply", token, output, "--module", standard, base, left, right));
  assert.equal(readFileSync(metadataAt(output), "utf8"), retired);
  assert.ok(!existsSync(join(output, "history", "value.loom")));
  const modified = join(directory, "modified"); cpSync(output, modified, { recursive: true, filter: () => true });
  rmSync(metadataAt(modified));
  const lost = review(output, modified, output);
  assert.equal(lost.status, 1);
  assert.match(lost.stdout + lost.stderr, /retired package history/);
  writeFileSync(join(output, "value_test.loom"), "test fn retained_edit() { assert amount() == 2 }\n");
  const binary = join(directory, `retired-tests${process.platform === "win32" ? ".exe" : ""}`);
  success(invoke(loom, ["test", output, "--no-run", "--output", binary]));
  success(invoke(binary, [], { ...environment, LOOM_GC_STRESS: "1" }));
}
