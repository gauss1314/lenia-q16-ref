# Explorer (Phase 0 prototype)

Single-file WebGPU float32 preview for founder play-testing. Not the integer
reference: it implements the continuous definition (spec §5 note) and is for
looking, not for measuring.

Open `index.html` from any local web server in desktop Chrome or Edge, e.g.
`python3 -m http.server` in this folder, then http://localhost:8000/.
Parameters go in the URL after `#`, for example
`#N=256&R=14&T=12&mu=0.16&sigma=0.017&beta=1/1`.

- Click the field to drop a random patch; "随机汤" drops one in the centre.
- "采集（导出状态）" downloads a `LQ16STAT` state record (spec §8.4) and a
  params line, which `lq16 run` accepts, so a find can be re-run on the
  integer reference.
- `steps/s` is the M4 preview speed on the machine running the page.
- `?test` runs compute only with no canvas (headless browsers cannot present
  WebGPU canvases); `window.__lq` exposes hooks for automated checks.

Checked 2026-10-31: on SwiftShader, 200 steps of a 128x128 field match the
float64 control (`tools/lenia_f64.py`) to max |diff| 7e-5.

No photosensitive safety mode yet (CP14 is a Phase 1 requirement); the
colour map is luminance-monotone and the page pauses when hidden.
