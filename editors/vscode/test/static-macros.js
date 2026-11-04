'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const { session } = require('./session');

async function staticMacrosSmoke(executable, stdRoot) {
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'loom-static-macro-editor-'));
  const folder = await fs.realpath(temporary);
  const file = path.join(folder, 'main.loom');
  const effect = path.join(folder, 'application-effect');
  const source = (operand, selected, expected) => `import std.reflect.Schema
import std.file.write_text
fn generate(comptime choice Int, types List[Schema]) Text {
    discard types
    if choice == ${selected} { "$0 + 1" } else { "$0" }
}
fn keep(value Int) Int
ensures result == value + ${expected} {
    generate!(${operand}, value)
}
fn main() {
    discard write_text(${JSON.stringify(effect)}, "APPLICATION_EFFECT")
}
`;
  const initial = source('1', 1, 1);
  await fs.writeFile(file, initial);
  const client = await session({ executable, stdRoot }, folder);
  try {
    const versions = [
      [initial, false],
      [source('2', 1, 1), true],
      [source('2', 1, 0), false],
      [source('2', 2, 0), true],
      [source('2', 2, 1), false],
      [source('value', 2, 1), true],
      [initial, false],
    ];
    for (let index = 0; index < versions.length; index++) {
      const version = index + 1;
      const [text, invalid] = versions[index];
      if (index === 0) await client.open(file, text, version);
      else await client.change(file, text, version);
      const report = await client.wait(file, version);
      assert.equal(report.diagnostics.length > 0, invalid, JSON.stringify(report));
      await assert.rejects(fs.access(effect), { code: 'ENOENT' });
    }
    assert.equal(await fs.readFile(file, 'utf8'), initial);
  } finally {
    await client.close();
    await fs.rm(temporary, { recursive: true, force: true });
  }
}

module.exports = { staticMacrosSmoke };
