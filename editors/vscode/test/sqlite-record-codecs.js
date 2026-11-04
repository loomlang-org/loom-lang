'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const { session } = require('./session');

async function sqliteRecordCodecsSmoke(executable, stdRoot) {
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'loom-canonical-editor-'));
  const folder = await fs.realpath(temporary);
  const file = path.join(folder, 'main.loom');
  const effect = path.join(folder, 'application-effect');
  const source = (relation, expected, visible = true) => `import std.storage.sqlite_record_codec
import std.storage.sqlite_codec_check
import std.storage.Table
import std.storage.Column
import std.storage.ValueKind
import std.storage.SqliteCheckError
import std.result.Result
import std.text.contains
import std.file.write_text

${visible ? 'pub ' : ''}type Amount = Int where self ${relation} 0
pub record Row {
    amount Amount
    enabled Bool
}
fn clause() Text {
    match sqlite_record_codec[Row](Table {
            id = "rows"
            physical_name = "rows"
            columns = [Column { id = "amount" physical_name = "amount" kind = ValueKind.Int nullable = false },
                Column { id = "enabled" physical_name = "flag" kind = ValueKind.Int nullable = false }]
        }) {
        Result.Ok(value) => sqlite_codec_check(value)
        Result.Err(SqliteCheckError.UnsupportedType) => "opaque"
        Result.Err(_) => "unexpected error"
    }
}
fn main() {
    comptime if contains(clause(), ${JSON.stringify(expected)}) {
    } else {
        let wrong Int = true
    }
    discard write_text(${JSON.stringify(effect)}, "APPLICATION_EFFECT")
}
`;
  const initial = source('>', '"amount" > 0');
  await fs.writeFile(path.join(folder, 'loom.toml'), '[module]\nname = "canonical_editor"\n');
  await fs.writeFile(file, initial);
  const client = await session({ executable, stdRoot }, folder);
  try {
    const versions = [
      [initial, false],
      [source('>=', '"amount" > 0'), true],
      [source('>=', '"amount" >= 0'), false],
      [source('>=', '"amount" >= 0', false), true],
      [source('>=', 'opaque', false), false],
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

module.exports = { sqliteRecordCodecsSmoke };
