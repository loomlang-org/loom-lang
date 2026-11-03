'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const compiler = require('../compiler');

async function moduleConfigurationSmoke(executable, stdRoot) {
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'loom-configured-'));
  const root = await fs.realpath(temporary);
  const folder = path.join(root, 'app');
  const codec = path.join(root, 'codec', 'main.loom');
  const settings = { executable, stdRoot };
  const write = async (name, text) => {
    const file = path.join(root, name);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.writeFile(file, text);
  };
  const configure = async mode => write('export/loom.toml', `[module]\nname='export'\n[dependencies.codec]\npath='../codec'\n[dependencies.codec.configuration]\nmode='${mode}'\n`);
  try {
    await write('app/loom.toml', "[module]\nname='app'\n[dependencies.search]\npath='../search'\n[dependencies.export]\npath='../export'\n");
    const application = 'import search.value\nimport export.value\nfn main() { assert search.value() == 42 && export.value() == 42 }\n';
    await write('app/main.loom', application);
    await write('codec/loom.toml', "[module]\nname='codec'\n[configuration]\nmode='default'\n");
    const source = `import std.build.configuration_text
pub fn value() Int ensures result == 42 {
    comptime if configuration_text("mode") == "bad" {
        false
    } else {
        apply(41)
    }
}
pub fn apply(input Int) Int ensures result == input + 1 { input + 1 }
`;
    await write('codec/main.loom', source);
    await write('search/loom.toml', "[module]\nname='search'\n[dependencies.codec]\npath='../codec'\n[dependencies.codec.configuration]\nmode='search'\n");
    for (const name of ['search', 'export']) {
      await write(`${name}/main.loom`, 'import codec.value\npub fn value() Int ensures result == 42 { codec.value() }\n');
    }
    await configure('export');
    const cold = await compiler.check(settings, folder, []);
    assert.deepEqual(cold.diagnostics, []);
    assert.equal(cold.analysisReused, false);
    assert.equal((await compiler.check(settings, folder, [])).analysisReused, true);
    const query = await compiler.query(settings, folder, path.join(folder, 'main.loom'), application.lastIndexOf('search.value()') + 7, []);
    assert.deepEqual(query.diagnostics, []);
    assert.ok(query.hover.types.some(type => type.includes('Int')));
    // An identical physical path cannot silently select the first instance.
    const ambiguous = await compiler.query(settings, folder, codec, source.lastIndexOf('input +'), []);
    assert.match(ambiguous.error, /multiple module instances/);
    const selected = await compiler.query(settings, path.join(root, 'search'), codec, source.lastIndexOf('input +'), []);
    assert.deepEqual(selected.diagnostics, []);
    assert.ok(selected.hover, JSON.stringify(selected));
    // No source edit or client invalidation notification: reload the manifest.
    await configure('bad');
    const bad = await compiler.check(settings, folder, []);
    assert.equal(bad.analysisReused, false);
    assert.ok(bad.diagnostics.length);
    await configure('export');
    const repaired = await compiler.check(settings, folder, []);
    assert.deepEqual(repaired.diagnostics, []);
    assert.equal((await compiler.check(settings, folder, [])).analysisReused, true);
    assert.equal(await fs.readFile(codec, 'utf8'), source);
    console.log('Module configuration editor smoke passed: distinct contexts, ambiguous-path refusal, manifest invalidation and repair.');
  } finally {
    compiler.close();
    await fs.rm(temporary, { recursive: true, force: true });
  }
}

module.exports = { moduleConfigurationSmoke };
if (require.main === module) {
  moduleConfigurationSmoke(path.resolve(process.env.LOOM_COMPILER || '../../target/loom'), path.resolve('../../compiler/std'))
    .catch(error => { console.error(error); process.exitCode = 1; });
}
