// node tools/wasm_run.mjs <wasm> <params.lq16> <state.lq16> <steps> <expected_chain> <expected_final> <name> [algo]
import fs from 'fs';
const [wasmPath, pPath, sPath, steps, chain, final, name, algo] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
const e = instance.exports;
const put = (bytes) => { const p = e.lq16_alloc(bytes.length); new Uint8Array(e.memory.buffer, p, bytes.length).set(bytes); return p; };
const pb = fs.readFileSync(pPath), sb = fs.readFileSync(sPath);
const pp = put(pb), sp = put(sb);
const t0 = Date.now();
const code = e.lq16_run(pp, pb.length, sp, sb.length, Number(steps), Number(algo || 0));
const res = new TextDecoder().decode(new Uint8Array(e.memory.buffer, e.lq16_result_ptr(), 256)).replace(/\0+$/, '');
if (code !== 0) { console.log(`wasm ${name} ERROR ${res}`); process.exit(1); }
const gotC = 'sha256:' + res.slice(0, 64), gotF = 'sha256:' + res.slice(64, 128);
const ok = gotC === chain && gotF === final;
console.log(`wasm ${name} ${ok ? 'ok' : 'MISMATCH ' + gotC} (${((Date.now() - t0) / 1000).toFixed(1)} s)`);
if (!ok) process.exit(1);
