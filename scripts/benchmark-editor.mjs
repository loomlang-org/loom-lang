#!/usr/bin/env node
// Real resident compiler requests with unsaved semantic edits, not cache hits.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { arch, platform, release, loadavg } from 'node:os';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const compiler = createRequire(import.meta.url)('../editors/vscode/compiler.js');
const options = { compiler: join(root, 'target/loom'), runs: '3', output: join(root, 'target/performance/editor.json') };
for (let index = 2; index < process.argv.length; index++) {
  const name = process.argv[index];
  if (name === '--help') {
    console.log('Usage: node scripts/benchmark-editor.mjs [--compiler path] [--baseline path] [--runs 1..20] [--output path]\nMeasures the full SMT example through the resident editor worker: cold check, two unsaved semantic body edits, invalid predicate, and restoration. Uses overlays; never edits checkout sources. Fresh sessions alternate baseline/candidate order. No timing threshold or stable speedup is inferred.');
    process.exit(0);
  }
  if (!['--compiler', '--baseline', '--runs', '--output'].includes(name) || !process.argv[index + 1]) {
    throw new Error(`Invalid benchmark option ${name}; use --help`);
  }
  options[name.slice(2)] = process.argv[++index];
}
const runs = Number(options.runs);
assert.ok(Number.isInteger(runs) && runs >= 1 && runs <= 20, '--runs must be 1..20');
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const variants = options.baseline ? ['baseline', 'compiler'] : ['compiler'];
const images = Object.fromEntries(await Promise.all(variants.map(async name => {
  const executable = resolve(options[name]);
  return [name, { executable, sha256: sha(await readFile(executable)) }];
})));
async function sources(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    if (entry.name === 'target') continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await sources(path));
    else if (entry.name.endsWith('.loom') || entry.name === 'loom.toml') {
      files.push([relative(root, path), sha(await readFile(path))]);
    }
  }
  return files;
}
const folder = join(root, 'compiler/examples/smt_contracts');
const file = join(folder, 'recursive_scans.loom');
const stdRoot = join(root, 'compiler/std');
const source = await readFile(file, 'utf8');
const inputs = await sources(folder);
inputs.push(...await sources(stdRoot));
const sourceSha256 = sha(JSON.stringify(inputs));
const replace = (from, to) => {
  const changed = source.replace(from, to);
  assert.notEqual(changed, source, `Benchmark fixture no longer contains ${from}`);
  return changed;
};
const edits = [
  ['cold', source, true],
  ['body-edit-1', replace('label = "sensor"', 'label = "sensor-edited"'), true],
  ['body-edit-2', replace('label = "sensor"', 'label = "sensor-again"'), true],
  ['invalid-predicate', replace('value.amount > 0', 'value.amount >= 0'), false],
  ['restored', source, true],
];
const report = {
  revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
  dirty: execFileSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).trim() !== '',
  host: { platform: platform(), arch: arch(), release: release() },
  solverVersion: execFileSync('z3', ['--version'], { encoding: 'utf8' }).trim(),
  initialLoad: loadavg(),
  images, sourceSha256, runs,
  method: 'Fresh editor sessions, alternating compiler order; unsaved semantic edits in the full SMT example. Includes overlay, reload, checking, proof and serialized worker transport; no unchanged-analysis samples.',
  samples: [],
};
try {
  for (let pair = 0; pair < runs; pair++) {
    for (const variant of pair % 2 ? [...variants].reverse() : variants) {
      const settings = { executable: images[variant].executable, stdRoot };
      try {
        for (const [name, text, valid] of edits) {
          const overlay = await compiler.snapshots([{ path: file, text }]);
          try {
            const start = performance.now();
            const result = await compiler.check(settings, folder, overlay.args);
            const wallMs = performance.now() - start;
            assert.equal(result.error, undefined);
            assert.equal(result.diagnostics.length === 0, valid, JSON.stringify(result.diagnostics));
            assert.equal(result.analysisReused, false, `${name} unexpectedly reused a whole analysis`);
            if (!valid) assert.ok(result.diagnostics.some(value => /proof|proved/.test(value.message)));
            const sample = { pair, variant, name, wallMs, definitionsReused: result.definitionsReused, bodiesReused: result.bodiesReused };
            report.samples.push(sample);
            console.log(JSON.stringify(sample));
          } finally { await overlay.dispose(); }
        }
      } finally { compiler.close(); }
    }
  }
  assert.equal(await readFile(file, 'utf8'), source);
  const current = await sources(folder);
  current.push(...await sources(stdRoot));
  assert.equal(sha(JSON.stringify(current)), sourceSha256, 'Source changed during measurement');
  for (const image of Object.values(images)) assert.equal(sha(await readFile(image.executable)), image.sha256);
  const median = values => {
    const sorted = [...values].sort((a, b) => a - b);
    return (sorted[Math.floor((sorted.length - 1) / 2)] + sorted[Math.floor(sorted.length / 2)]) / 2;
  };
  report.medians = variants.flatMap(variant => edits.map(([name]) => ({ variant, name,
    wallMs: median(report.samples.filter(value => value.variant === variant && value.name === name).map(value => value.wallMs)),
  })));
  report.finalLoad = loadavg();
  const output = resolve(options.output);
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`Raw measurements: ${output}`);
} finally { compiler.close(); }
