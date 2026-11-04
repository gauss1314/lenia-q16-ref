"""Builds web/vectors/standalone.html: one self-contained page that runs the public
vectors v0 and bench/v0 in the browser (WASM in a Web Worker) with every input
embedded, so it can be opened straight from disk (file://) in desktop Chrome.

  tools/prepare_vec_tmp.sh && python3 tools/make_standalone.py
  python3 tools/make_standalone.py --check   (CI: the committed page lists the same
                                             items, params, steps and hashes as the tsv files)

Re-run whenever the WASM build, vectors/v0 or bench/v0 change.
"""
import base64, json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
PAGE = os.path.join(ROOT, 'web/vectors/standalone.html')


def rows(path):
    return [l.rstrip('\n').split('\t') for l in open(os.path.join(ROOT, path)) if l.strip() and not l.startswith('#')]


if '--check' in sys.argv:
    page = open(PAGE).read()
    emb = json.loads(page.split('const ITEMS = ', 1)[1].split(';\nconst WASM', 1)[0])
    want = [(r[0], r[1], int(r[6]), r[7], r[8]) for r in rows('vectors/v0/vectors.tsv')]
    want += [('bench-' + r[0], r[1], int(r[3]), r[4], None) for r in rows('bench/v0/benchmarks.tsv')]
    got = [(e['name'], e['params'], e['steps'], e['chain'], e['final']) for e in emb]
    spec = open(os.path.join(ROOT, 'src/params.rs')).read().split('SPEC_ID: &str = "')[1].split('"')[0]
    if got != want or f'const SPEC = "{spec}"' not in page:
        sys.exit('web/vectors/standalone.html is stale: run tools/prepare_vec_tmp.sh && python3 tools/make_standalone.py')
    print('standalone.html matches vectors/v0 and bench/v0')
    sys.exit(0)
b64 = lambda p: base64.b64encode(open(os.path.join(ROOT, p), 'rb').read()).decode()

items = []
for name, params, n, sz, amp, seed, steps, chain, final in rows('vectors/v0/vectors.tsv'):
    items.append(dict(name=name, params=params, algo=0, steps=int(steps), chain=chain, final=final,
                      p=b64(f'vec-tmp/{name}/params.lq16'), s=b64(f'vec-tmp/{name}.state')))
for r in rows('bench/v0/benchmarks.tsv'):
    name, params, state, steps, chain = r[:5]
    items.append(dict(name='bench-' + name, params=params, algo=1, steps=int(steps), chain=chain, final=None,
                      p=b64(f'vec-tmp/bench-{name}/params.lq16'), s=b64(f'bench/v0/{state}')))
wasm = b64('target/wasm32-unknown-unknown/release/lq16_wasm.wasm')
spec = open(os.path.join(ROOT, 'src/params.rs')).read().split('SPEC_ID: &str = "')[1].split('"')[0]

html = '''<!DOCTYPE html>
<html lang="zh"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>lenia-q16 确定性自测</title>
<style>
 :root { --bg:#111418; --text:#e6e8eb; --muted:#9aa3ad; --ok:#7bd88f; --bad:#ff8a7a; }
 body { margin:0; padding:16px; background:var(--bg); color:var(--text); font:14px/1.5 system-ui, sans-serif; }
 h1 { font-size:18px; margin:0 0 8px; } .muted { color:var(--muted); }
 table { border-collapse:collapse; margin:12px 0; } td, th { padding:3px 10px; border-bottom:1px solid #2a3240; text-align:left; font-family:ui-monospace, monospace; font-size:13px; }
 .ok { color:var(--ok); } .bad { color:var(--bad); }
 textarea { width:100%; max-width:760px; height:140px; background:#0d1014; color:var(--text); border:1px solid #333a44; border-radius:4px; font:12px ui-monospace, monospace; }
 button { background:#2a3240; color:var(--text); border:1px solid #3a4454; border-radius:4px; padding:5px 10px; cursor:pointer; }
</style></head><body>
<h1>lenia-q16 确定性自测（规范 __SPEC__）</h1>
<p class="muted">在这台电脑的浏览器里，用 WASM 整数版跑公开测试向量和基准结构，和参考哈希逐项比对。全部跑完约 10–20 分钟，期间请保持本页在前台。跑完把下方方框里的结果复制给 AI 代理。</p>
<table id="t"><tr><th>项目</th><th>步数</th><th>结果</th><th>秒</th></tr></table>
<p id="status">准备中…</p>
<textarea id="out" readonly></textarea><br><button id="copy">复制结果</button>
<script>
const ITEMS = __ITEMS__;
const WASM = "__WASM__";
const SPEC = "__SPEC__";
const $ = id => document.getElementById(id);
const worker = new Worker(URL.createObjectURL(new Blob([`
  const dec = b => Uint8Array.from(atob(b), c => c.charCodeAt(0));
  let e;
  onmessage = async ({ data }) => {
    if (data.wasm) { e = (await WebAssembly.instantiate(dec(data.wasm), {})).instance.exports; postMessage({ ready: true }); return; }
    const it = data.item, pb = dec(it.p), sb = dec(it.s);
    const put = b => { const p = e.lq16_alloc(b.length); new Uint8Array(e.memory.buffer, p, b.length).set(b); return p; };
    const t0 = performance.now();
    const code = e.lq16_run(put(pb), pb.length, put(sb), sb.length, it.steps, it.algo);
    const res = new TextDecoder().decode(new Uint8Array(e.memory.buffer, e.lq16_result_ptr(), 256)).replace(/\\0+$/, '');
    postMessage({ code, res, seconds: (performance.now() - t0) / 1000 });
  };`], { type: 'text/javascript' })));
const results = [];
function row(it) { const tr = $('t').insertRow(); tr.innerHTML = `<td>${it.name}</td><td>${it.steps}</td><td>运行中…</td><td></td>`; return tr; }
function next(i) {
  if (i === ITEMS.length) {
    const ok = results.every(r => r.ok);
    $('status').innerHTML = ok ? '<b class="ok">全部一致</b>' : '<b class="bad">有不一致</b>';
    $('out').value = JSON.stringify({ spec: SPEC, userAgent: navigator.userAgent, when: new Date().toISOString(), allOk: ok, results }, null, 1);
    return;
  }
  const it = ITEMS[i], tr = row(it);
  $('status').textContent = `正在跑第 ${i + 1}/${ITEMS.length} 项：${it.name}`;
  worker.onmessage = ({ data }) => {
    const chain = data.code === 0 ? 'sha256:' + data.res.slice(0, 64) : null;
    const final = data.code === 0 ? 'sha256:' + data.res.slice(64, 128) : null;
    const ok = chain === it.chain && (it.final === null || final === it.final);
    results.push({ name: it.name, ok, seconds: +data.seconds.toFixed(1), got: chain || data.res });
    tr.cells[2].innerHTML = ok ? '<span class="ok">一致</span>' : '<span class="bad">不一致</span>';
    tr.cells[3].textContent = data.seconds.toFixed(1);
    next(i + 1);
  };
  worker.postMessage({ item: it });
}
worker.onmessage = () => next(0);
worker.postMessage({ wasm: WASM });
$('copy').onclick = () => { $('out').select(); navigator.clipboard ? navigator.clipboard.writeText($('out').value) : document.execCommand('copy'); };
</script></body></html>
'''
html = html.replace('__ITEMS__', json.dumps(items)).replace('__WASM__', wasm).replace('__SPEC__', spec)
open(PAGE, 'w').write(html)
print(PAGE, len(html), 'bytes,', len(items), 'items')
