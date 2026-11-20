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
  // It becomes a semantic baseline below, so account for the new root test.
  success(change("refresh", output));

  const revived = join(directory, "revived"), unchanged = join(directory, "unchanged");
  cpSync(output, revived, { recursive: true, filter: () => true });
  cpSync(output, unchanged, { recursive: true, filter: () => true });
  const revivedPackage = join(revived, "history");
  writeFileSync(join(revivedPackage, "value.loom"), `
concept Value { fn value(self Self) Int }
record Marker {}
impl Value for Marker { fn value(self Marker) Int { 9 } }
fn source() Int { 9 }
fn copied() Int { source() }
`);
  writeFileSync(join(revivedPackage, "value_test.loom"), "test fn private_restored_member() {\n    assert Marker {}.value() == 9\n    assert copied() == 9\n}\n");
  success(change("refresh", revivedPackage, "--derive", parent, "value.loom", "copied", "-"));
  const fresh = readFileSync(metadataAt(revived), "utf8");
  const freshSource = fresh.match(/^id (\S+) fn source /m)?.[1];
  assert.ok(freshSource && freshSource !== parent);
  assert.match(fresh, new RegExp(`^origin ${child} ${parent}$`, "m"));
  const next = Number(fresh.match(/^next (\d+)$/m)?.[1]);
  const oldNext = Number(retired.match(/^next (\d+)$/m)?.[1]);
  assert.ok(next > oldNext);
  const restoredPreview = success(review(output, revived, unchanged));
  const restoredToken = restoredPreview.match(/^review token: ([a-f0-9]{64})$/m)?.[1]; assert.ok(restoredToken);
  const restored = join(directory, "restored");
  success(change("--apply", restoredToken, restored, "--module", standard, output, revived, unchanged));
  // Reviewed output canonically orders IDs; every complete row must survive.
  assert.deepEqual(readFileSync(metadataAt(restored), "utf8").split("\n").sort(), fresh.split("\n").sort());
  success(invoke(loom, ["test", join(restored, "history"), "--no-run", "--output", binary]));
  success(invoke(binary, [], { ...environment, LOOM_GC_STRESS: "1" }));
  writeFileSync(metadataAt(revived), fresh.replace(`next ${next}\n`, `next ${oldNext}\n`));
  const reset = review(output, revived, unchanged);
  assert.equal(reset.status, 1);
  assert.match(reset.stdout + reset.stderr, /retained allocator/);
  const staleRestoration = join(directory, "stale restoration");
  assert.equal(change("--apply", restoredToken, staleRestoration, "--module", standard, output, revived, unchanged).status, 1);
  assert.ok(!existsSync(staleRestoration));
  writeFileSync(metadataAt(revived), fresh);
}
