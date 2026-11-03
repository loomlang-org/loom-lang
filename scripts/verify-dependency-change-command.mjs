import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";

// Exercise complete checked binding sets through the delivered read-only CLI.
export function verifyDependencyChangeCommand(loom, directory, environment, standard) {
  mkdirSync(directory);
  function command(args, expected = 0) {
    const result = spawnSync(loom, ["change", ...args], {
      cwd: directory, env: environment, encoding: "utf8", maxBuffer: 4 * 1024 * 1024,
    });
    if (result.error) throw result.error;
    assert.equal(result.status, expected, result.stderr);
    return expected === 0 ? result.stdout : result.stderr;
  }
  function inventory(root = directory) {
    return readdirSync(root, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name)).flatMap((entry) => {
      const file = join(root, entry.name);
      return entry.isDirectory() ? inventory(file) : [[relative(directory, file), createHash("sha256").update(readFileSync(file)).digest("hex")]];
    });
  }
  const api = `pub concept Read { fn read(self Self) Int }
impl Read for Int { fn read(self Int) Int { self + 1 } }
impl Read for Bool { fn read(self Bool) Int { 7 } }
`;
  for (const name of ["library", "fork"]) {
    const provider = join(directory, name);
    mkdirSync(provider);
    writeFileSync(join(provider, "loom.toml"), '[module]\nname="library"\n');
    writeFileSync(join(provider, "api.loom"), api);
  }
  const before = join(directory, "before 雪");
  const after = join(directory, "after $(literal)");
  mkdirSync(join(before, "child"), { recursive: true });
  function manifest(dependency, mode = "search") {
    return `[module]\nname="sample"\n[dependencies.library]\npath="../${dependency}"\n[dependencies.library.configuration]\nmode="${mode}"\n`;
  }
  writeFileSync(join(before, "loom.toml"), manifest("library"));
  writeFileSync(join(before, "main.loom"), `import library.Read
fn generic_read[T Read](value T) Int { value.read() }
fn main() { assert false
    assert generic_read(7) > 0 && generic_read(true) > 0 }
`);
  writeFileSync(join(before, "child", "api.loom"), `import library.Read
fn child_read[T Read](value T) Int { value.read() }
fn child() Int { child_read(7) + child_read(true) }
`);
  writeFileSync(join(before, "child", "api_test.loom"), "test fn private_scope() { assert false\n    assert child() > 0 }\n");
  command(["init", "sample", before]);
  command(["init", "sample.child", join(before, "child")]);
  cpSync(before, after, { recursive: true, filter: () => true });
  function review() {
    const inputs = inventory();
    const output = command(["--dependencies", standard, before, after]);
    assert.deepEqual(inventory(), inputs, "dependency review rewrote inputs or produced files");
    assert.doesNotMatch(output, /^review token:/m);
    assert.match(output, /not behavioral compatibility/);
    return output;
  }
  assert.match(review(), /changed binding observations: 0\n/);
  writeFileSync(join(after, "loom.toml"), manifest("fork"));
  const fork = review();
  assert.match(fork, /view lexical/);
  assert.match(fork, /view observed instances\n  before targets: 2\n/);
  assert.match(fork, /  after targets: 2\n/);
  assert.match(fork, /test root=child package=child/);
  const portable = fork.replaceAll("\\", "/");
  assert.ok(["library", "fork"].every(name => portable.includes(join(directory, name).replaceAll("\\", "/"))));
  writeFileSync(join(after, "loom.toml"), manifest("library", "export"));
  const configured = review();
  assert.match(configured, /view observed instances\n  before targets: 2\n/);
  assert.ok(configured.includes('mode="search"') && configured.includes('mode="export"'));
  writeFileSync(join(after, "loom.toml"), manifest("fork"));
  writeFileSync(join(directory, "fork", "api.loom"), `pub concept Read { fn read(self Self) Int { 7 } }
impl Read for Int {}
impl Read for Bool {}
`);
  const defaults = review();
  assert.match(defaults, /view observed instances\n  before targets: 2\n(?:    .*\n){2}  after targets: 0\n/);
  const application = join(after, "main.loom");
  writeFileSync(application, readFileSync(application, "utf8").replace("generic_read(7)", "generic_read(8)"));
  const invalid = inventory();
  assert.match(command(["--dependencies", standard, before, after], 1), /unchanged application source/);
  assert.deepEqual(inventory(), invalid, "rejected dependency review changed inputs");
  return { fork, configured, defaults };
}
