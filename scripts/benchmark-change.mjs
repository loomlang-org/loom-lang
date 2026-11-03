#!/usr/bin/env node
// Paired native previews with real edits, matching tokens and a contract canary.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {cpSync,existsSync,mkdirSync,mkdtempSync,readFileSync,readdirSync,realpathSync,rmSync,writeFileSync} from 'node:fs';
import {arch,cpus,platform,release,tmpdir} from 'node:os';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const options={compiler:join(root,'target/loom'),runs:'3',sizes:'50,200',output:join(root,'target/performance/change.json')};
for(let index=2;index<process.argv.length;index++){
  const option=process.argv[index];
  if(option==='--help'){
    console.log('Usage: node scripts/benchmark-change.mjs --baseline path [--compiler path] [--runs 1..20] [--sizes N,N] [--output path]\nmacOS only. Fresh native loom change previews alternate baseline/candidate order on identical edited snapshots. Each pair changes called helpers, requires matching review tokens and read-only inputs. Both tools must reject an invalid helper contract. Compiler and companion hashes, wall time and peak RSS are recorded. Temporary inputs are removed.');
    process.exit(0);
  }
  if(!['--compiler','--baseline','--runs','--sizes','--output'].includes(option)||!process.argv[index+1]||process.argv[index+1].startsWith('--'))throw new Error(`Invalid option ${option}; use --help`);
  options[option.slice(2)]=process.argv[++index];
}
assert.ok(options.baseline,'--baseline is required');
assert.equal(platform(),'darwin','this benchmark uses macOS /usr/bin/time -l');
const runs=Number(options.runs),sizes=options.sizes.split(',').map(Number);
assert.ok(Number.isInteger(runs)&&runs>=1&&runs<=20,'--runs must be 1..20');
assert.ok(sizes.length>=1&&sizes.length<=8&&new Set(sizes).size===sizes.length&&sizes.every(n=>Number.isInteger(n)&&n>=2&&n<=512),'--sizes needs distinct helper counts in 2..512');
const hash=file=>createHash('sha256').update(readFileSync(file)).digest('hex');
function describeTool(value){
  const compiler=realpathSync(resolve(value));
  const candidates=[join(dirname(compiler),'debug/loom-change'),resolve(dirname(compiler),'../lib/loom/loom-change')].filter(existsSync);
  assert.equal(candidates.length,1,`Use a compiler from one complete checkout or installed toolchain: ${compiler}`);
  const companion=realpathSync(candidates[0]);
  return {compiler,compilerSha256:hash(compiler),companion,companionSha256:hash(companion)};
}
const tools={baseline:describeTool(options.baseline),candidate:describeTool(options.compiler)};
const directory=realpathSync.native(mkdtempSync(join(tmpdir(),'loom-change-benchmark-')));
const environment={...process.env};delete environment.LOOM_GC_STRESS;
const report={schema:1,tools,runs,sizes,host:{platform:platform(),arch:arch(),release:release(),cpu:cpus()[0]?.model},
  method:'Fresh public native loom change processes; same actually edited inputs per pair; alternating order after one warmup per tool; matching changing review tokens, unchanged input bytes and rejected bad contracts required. Main asserts false and must never run. macOS RSS is a process peak, not summed simultaneous memory. No persistent frontend cache. Temporary inputs are removed.',cases:[]};
function invoke(name,args,timed=false,expected=0){
  const executable=tools[name].compiler,start=process.hrtime.bigint();
  const result=spawnSync(timed?'/usr/bin/time':executable,timed?['-l',executable,...args]:args,{cwd:directory,env:environment,encoding:'utf8',maxBuffer:4*1024*1024});
  if(result.error)throw result.error;
  assert.equal(result.status,expected,result.stdout+result.stderr);
  const token=result.stdout.match(/^review token: ([a-f0-9]{64})$/m)?.[1];
  if(timed)assert.ok(token,'missing public review token');
  if(expected===1)assert.match(result.stdout+result.stderr,/proof|postcondition/);
  const rss=timed?result.stderr.match(/(\d+)\s+maximum resident set size/):null;
  if(timed)assert.ok(rss,'missing macOS peak RSS');
  return {wallMs:Number(process.hrtime.bigint()-start)/1e6,rssBytes:rss?Number(rss[1]):null,token};
}
function inventory(path){return readdirSync(path).sort().map(name=>[name,hash(join(path,name))]);}
const median=values=>{const sorted=[...values].sort((a,b)=>a-b);return (sorted[Math.floor((sorted.length-1)/2)]+sorted[Math.floor(sorted.length/2)])/2;};
try{
  for(const size of sizes){
    const paths=['base','left','right'].map(name=>join(directory,`${size}-${name}`));mkdirSync(paths[0]);
    const helpers=Array.from({length:size},(_,i)=>i===0?'fn helper_0(input Int) Int ensures result == input { input }':`fn helper_${i}(input Int) Int { input + ${i} }`).join('\n');
    const source=helpers+'\nfn first(input Int) Int { helper_0(input) }\nfn second(input Int) Int { helper_1(input) }\nfn main() { assert false }\n';
    writeFileSync(join(paths[0],'main.loom'),source);invoke('candidate',['change','init','sample',paths[0]]);
    for(const path of paths.slice(1))cpSync(paths[0],path,{recursive:true,filter:()=>true});
    function edit(n){writeFileSync(join(paths[1],'main.loom'),source.replace('helper_0(input)',`helper_0(input + ${n})`));writeFileSync(join(paths[2],'main.loom'),source.replace('helper_1(input)',`helper_1(input + ${n+1})`));}
    edit(1);for(const name of ['baseline','candidate'])invoke(name,['change',...paths]);
    const samples=[],previous={};
    for(let pair=0;pair<runs;pair++){
      edit(pair+3);const before=paths.map(inventory),tokens=[];
      for(const name of pair%2?['candidate','baseline']:['baseline','candidate']){
        const sample=invoke(name,['change',...paths],true);assert.notEqual(sample.token,previous[name]);previous[name]=sample.token;tokens.push(sample.token);samples.push({pair,variant:name,...sample});
      }
      assert.equal(tokens[0],tokens[1]);assert.deepEqual(paths.map(inventory),before,'preview changed source inputs');
    }
    const valid=readFileSync(join(paths[1],'main.loom'),'utf8');
    const invalid=valid.replace('ensures result == input { input }','ensures result == input { input + 1 }');assert.notEqual(invalid,valid);writeFileSync(join(paths[1],'main.loom'),invalid);
    const before=paths.map(inventory);for(const name of ['baseline','candidate'])invoke(name,['change',...paths],false,1);assert.deepEqual(paths.map(inventory),before);writeFileSync(join(paths[1],'main.loom'),valid);
    const summaries=Object.fromEntries(['baseline','candidate'].map(name=>{const rows=samples.filter(s=>s.variant===name);return [name,{wallMs:median(rows.map(s=>s.wallMs)),rssBytes:median(rows.map(s=>s.rssBytes))}];}));
    report.cases.push({size,summaries,samples,invalidContract:'both rejected without source writes'});console.log(JSON.stringify({size,summaries}));
  }
  for(const tool of Object.values(tools)){assert.equal(hash(tool.compiler),tool.compilerSha256);assert.equal(hash(tool.companion),tool.companionSha256);}
  const output=resolve(options.output);mkdirSync(dirname(output),{recursive:true});writeFileSync(output,JSON.stringify(report,null,2)+'\n');console.log(`Raw measurements: ${output}`);
}finally{rmSync(directory,{recursive:true,force:true});}
