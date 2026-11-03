import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { verifyMemberChangeCommand } from "./verify-member-change-command.mjs";
import { verifyDependencyChangeCommand } from "./verify-dependency-change-command.mjs";
import { verifyBodyChangeCommand } from "./verify-body-change-command.mjs";

// Exercise the delivered command, including argument forwarding and child exit
// status, from a directory outside the checkout and toolchain.
export function verifyChangeCommand(loom, fixtures, directory, environment, standard) {
  mkdirSync(directory);
  directory = realpathSync(directory);
  const inputs = ["base", "left", "right"].map((name, index) => {
    const target = join(directory, `${name} ${index} $(literal) ; 雪`);
    // Match staging's filtered traversal on Windows Unicode paths.
    cpSync(join(fixtures, name), target, { recursive: true, filter: () => true });
    return target;
  });
  function invoke(args) {
    const result = spawnSync(loom, ["change", ...args], {
      cwd: directory, env: environment, encoding: "utf8", maxBuffer: 4 * 1024 * 1024,
    });
    if (result.error) throw result.error;
    return result;
  }
  const preview = invoke(inputs);
  assert.equal(preview.status, 0, preview.stderr);
  const token = preview.stdout.match(/^review token: ([a-f0-9]{64})$/m)?.[1];
  assert.ok(token, "missing semantic review token");
  const rejected = join(directory, "rejected");
  assert.equal(invoke(["--apply", "stale", rejected, ...inputs]).status, 1);
  assert.ok(!existsSync(rejected), "stale review created output");
  assert.equal(invoke(["--apply"]).status, 2, "lost child usage status");
  const output = join(directory, "merged source");
  const applied = invoke(["--apply", token, output, ...inputs]);
  assert.equal(applied.status, 0, applied.stderr);
  const identities = readFileSync(join(output, ".loom-ids"), "utf8");
  assert.match(identities, /id amount-id fn amount app\.loom/);
  assert.match(readFileSync(join(output, "app.loom"), "utf8"), /amount may be revised independently/);
  writeFileSync(join(output, "merge_test.loom"), "test fn moved_definition_keeps_the_other_branches_edit() {\n    assert entry() == 2\n}\n");
  const tested = spawnSync(loom, ["test", output], {
    cwd: directory, env: environment, encoding: "utf8", maxBuffer: 4 * 1024 * 1024,
  });
  if (tested.error) throw tested.error;
  assert.equal(tested.status, 0, tested.stderr);
  assert.equal(tested.stdout, "1 tests passed\n");
  verifyMemberChangeCommand(loom, join(directory, "owned methods"), environment, standard);
  verifyBodyChangeCommand(loom, join(directory, "composed bodies"), environment, standard);
  if (standard) verifyDependencyChangeCommand(loom, join(directory, "dependency review"), environment, standard);
  return { inputs, token };
}
