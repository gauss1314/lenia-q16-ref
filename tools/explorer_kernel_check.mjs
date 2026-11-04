// node tools/explorer_kernel_check.mjs
// Checks the explorer's JavaScript port of the spec kernel (CH9) against the Rust reference:
// for a set of rule points, SHA-256 of the kernel record must equal `lq16 info`'s kernel hash.
import { readFileSync } from 'fs';
import { execFileSync } from 'child_process';
import { createHash } from 'crypto';
const here = new URL('..', import.meta.url).pathname;
const html = readFileSync(here + 'web/explorer/index.html', 'utf8');
const code = html.split('// BEGIN SPEC-KERNEL')[1].split('\n').slice(1).join('\n').split('// END SPEC-KERNEL')[0];
const specKernel = new Function(code + '; return specKernel;')();
const spec = readFileSync(here + 'src/params.rs', 'utf8').split('SPEC_ID: &str = "')[1].split('"')[0];
const cases = [];
for (const beta of ['1/1', '1/2,1/1', '1/1,1/3', '1/2,1/1,2/3', '1/4,3/4,1/1,5/12'])
  for (const R of [5, 7, 13, 14, 20, 26, 39, 50]) cases.push([R, beta]);
let bad = 0;
for (const [R, beta] of cases) {
  const params = `R=${R} T=10 mu=1500 sigma=150 beta=${beta}`;
  const want = execFileSync(here + 'target/release/lq16', ['info', params]).toString().match(/kernel\s+(sha256:\w+)/)[1];
  const w = specKernel(R, beta.split(',').map(t => t.split('/').map(Number)));
  const rec = Buffer.alloc(44 + 4 * w.length);
  rec.write('LQ16KERN', 0, 'ascii'); rec.write(spec, 8, 'ascii'); rec.writeUInt32LE(R, 40);
  w.forEach((x, i) => rec.writeUInt32LE(x, 44 + 4 * i));
  const got = 'sha256:' + createHash('sha256').update(rec).digest('hex');
  if (got !== want) { bad++; console.log('MISMATCH', params, got, want); }
}
console.log(bad ? `${bad} of ${cases.length} kernels differ` : `all ${cases.length} kernels match the Rust reference`);
process.exit(bad ? 1 : 0);
