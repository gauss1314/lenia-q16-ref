# Explorer (Phase 0 prototype)

Single-file WebGPU float32 preview for founder play-testing. Not the integer
reference: it implements the continuous definition (spec §5 note) and is for
looking, not for measuring.

Open `index.html` from any local web server in desktop Chrome or Edge, e.g.
`python3 -m http.server` in this folder, then http://localhost:8000/.
Parameters go in the URL after `#`, for example
`#N=256&R=13&T=10&mu=0.1923&sigma=0.0277&beta=1/2,1/1`.

- Click the field to drop a random patch; "随机汤" drops one in the centre.
- "采集（导出状态）" downloads a `LQ16STAT` state record (spec §8.4) and a
  params line, which `lq16 run` accepts, so a find can be re-run on the
  integer reference.
- `steps/s` is the M4 preview speed on the machine running the page.
- "预览核" switches the preview kernel (CH9): the continuous formula, or the
  spec integer kernel (spec §5, weights / 2^20) computed in the page by a
  JavaScript port of the spec algorithm (BigInt). CI checks that port against
  the Rust reference on 40 rule points (`tools/explorer_kernel_check.mjs`).
  The choice is in the URL (`kernel=int`) and in every logged event.
- The panel plots the total mass (last 300 samples, one per second) and the
  centroid track on the torus (mass-weighted circular mean). Both reset on
  "应用参数", "随机汤" and "清空".
- Session log (play log, G8): parameter changes, soups, collects, pauses, an M4
  sample every 10 s while running, and free-text notes ("记一笔", Enter to save)
  are kept in the browser (localStorage) across reloads. "导出试玩日志"
  downloads them as `playlog-<start time>.json`, with the GPU adapter and the
  median steps/s; "开始新日志" starts a fresh one.
- `?test` runs compute only with no canvas (headless browsers cannot present
  WebGPU canvases); `window.__lq` exposes hooks for automated checks.

Checked 2026-10-31: on SwiftShader, 200 steps of a 128x128 field match the
float64 control (`tools/lenia_f64.py`) to max |diff| 7e-5.

No photosensitive safety mode yet (CP14 is a Phase 1 requirement); the
colour map is luminance-monotone and the page pauses when hidden.
