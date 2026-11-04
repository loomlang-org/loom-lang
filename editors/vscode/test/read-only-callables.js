'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { URI } = require('vscode-uri');
const { TextDocument } = require('vscode-languageserver-textdocument');
const { session } = require('./session');

async function readOnlyCallablesSmoke(executable, stdRoot) {
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'loom-read-only-'));
  const folder = await fs.realpath(temporary), file = path.join(folder, 'main.loom');
  const source = `import std.reflect.functions.read_only_callable
fn observe(values List[Int]) Int { values[0] }
fn admitted() Bool ensures result { read_only_callable(observe) }
fn main() {
    let capable = read_only_callable(observe)
    assert capable
    assert admitted()
}
`;
  await fs.writeFile(file, source);
  const client = await session({ executable, stdRoot }, folder);
  try {
    await client.open(file, source);
    assert.deepEqual((await client.wait(file, 1)).diagnostics, []);
    const document = TextDocument.create(URI.file(file).toString(), 'loom', 1, source);
    const position = document.positionAt(source.indexOf('assert capable') + 'assert '.length);
    const params = { textDocument: { uri: document.uri }, position };
    const hover = await client.rpc.sendRequest('textDocument/hover', params);
    assert.ok(hover.contents.some(item => item.value === 'Bool'), JSON.stringify(hover));
    const definitions = await client.rpc.sendRequest('textDocument/definition', params);
    assert.equal(definitions.length, 1);
    assert.equal(definitions[0].uri, document.uri);
    assert.equal(document.getText(definitions[0].range), 'capable');
    const changed = source.replace('{ values[0] }', '{ values[0] = 0\nvalues[0] }');
    assert.notEqual(changed, source);
    await client.change(file, changed, 2);
    const rejected = await client.wait(file, 2);
    assert.ok(rejected.diagnostics.some(item => /proof|postcondition|proved/.test(item.message)), JSON.stringify(rejected));
    await client.change(file, source, 3);
    assert.deepEqual((await client.wait(file, 3)).diagnostics, []);
    const repaired = await client.rpc.sendRequest('textDocument/hover', params);
    assert.ok(repaired.contents.some(item => item.value === 'Bool'), JSON.stringify(repaired));
    assert.equal(await fs.readFile(file, 'utf8'), source);
  } finally {
    await client.close();
    await fs.rm(temporary, { recursive: true, force: true });
  }
}

module.exports = { readOnlyCallablesSmoke };
