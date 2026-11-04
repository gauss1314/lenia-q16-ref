"""M7 phantom rate (Phase 0, not normative): re-run collected explorer candidates on the
integer reference.

  python3 tools/m7_check.py <state.lq16> <params.txt> [<state.lq16> <params.txt> ...] [--kernel cont|int]
  python3 tools/m7_check.py --log playlog.json <dir with the collected files>

Per candidate (collection handoff CH4, CH5; measurement report M7):
  1. The explorer already quantised the field with round-half-even (q16-rne/1).
  2. Target = the largest 8-connected component (on the torus) of cells >= 0.1.
  3. Body e = the larger side of its bounding box; margin m = 2R; N = the smallest legal
     size in {128, 256, 512} with N >= e + 2m, else "too_large".
  4. The target is centred in an N x N zero torus; cells outside the target mask dilated by
     R (Chebyshev) are zeroed, and so are other components' mask cells inside it.
  5. Run 2000*T steps (NTT). Survives = final mass > 1 cell and < 25% of the torus above 0.01.
Prints one JSON line per candidate and the phantom rate (share that does not survive).
"""
import json, os, subprocess, sys, tempfile
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from lenia_f64 import read_state, state_record

LQ16 = os.path.join(HERE, '..', 'target', 'release', 'lq16')
MASK = round(0.1 * 65535)


def components(mask):
    """Labels 8-connected components on the torus; returns (labels, sizes)."""
    n = mask.shape[0]
    lab = np.zeros(mask.shape, np.int32)
    sizes = [0]
    for y0, x0 in zip(*np.nonzero(mask)):
        if lab[y0, x0]:
            continue
        k = len(sizes)
        lab[y0, x0] = k
        stack, size = [(y0, x0)], 0
        while stack:
            y, x = stack.pop()
            size += 1
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    yy, xx = (y + dy) % n, (x + dx) % n
                    if mask[yy, xx] and not lab[yy, xx]:
                        lab[yy, xx] = k
                        stack.append((yy, xx))
        sizes.append(size)
    return lab, sizes


def crop(a, R):
    n = a.shape[0]
    lab, sizes = components(a >= MASK)
    if len(sizes) == 1:
        return None, 'empty', None
    k = int(np.argmax(sizes[1:])) + 1
    tgt = lab == k
    # unwrap: move the target's circular centroid to the middle
    ang = 2 * np.pi * np.arange(n) / n
    cy = np.angle((tgt.sum(1) * np.exp(1j * ang)).sum()) * n / (2 * np.pi)
    cx = np.angle((tgt.sum(0) * np.exp(1j * ang)).sum()) * n / (2 * np.pi)
    sh = (n // 2 - int(round(cy)) % n, n // 2 - int(round(cx)) % n)
    a, lab, tgt = (np.roll(v, sh, (0, 1)) for v in (a, lab, tgt))
    ys, xs = np.nonzero(tgt)
    e = int(max(ys.max() - ys.min() + 1, xs.max() - xs.min() + 1))
    N = next((s for s in (128, 256, 512) if s >= e + 4 * R), None)
    if N is None:
        return None, 'too_large', e
    # keep region: target mask dilated by R (Chebyshev), other components' cells removed
    keep = tgt.copy()
    for _ in range(R):
        k2 = keep.copy()
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                k2 |= np.roll(keep, (dy, dx), (0, 1))
        keep = k2
    keep &= ~((lab > 0) & ~tgt)
    b = np.where(keep, a, 0)
    # bounding-box centre to the middle of the N x N torus
    by, bx = (ys.min() + ys.max()) // 2, (xs.min() + xs.max()) // 2
    out = np.zeros((N, N), np.uint16)
    h = min(N, n) // 2
    for oy in range(-h, h):
        out[N // 2 + oy, N // 2 - h:N // 2 + h] = b[(by + oy) % n, [(bx + ox) % n for ox in range(-h, h)]]
    return out, 'ok', e


def check(state_path, params_path, kernel):
    params = open(params_path).read().split('\n')[0].strip()
    d = dict(kv.split('=') for kv in params.split())
    R, T = int(d['R']), int(d['T'])
    a = read_state(state_path)
    s0, status, e = crop(a, R)
    row = dict(file=os.path.basename(state_path), params=params, kernel=kernel, body=e, status=status)
    if s0 is None:
        row['survived'] = None
        return row
    with tempfile.TemporaryDirectory() as tmp:
        p0, p1 = os.path.join(tmp, 's0.lq16'), os.path.join(tmp, 'end.lq16')
        open(p0, 'wb').write(state_record(s0))
        out = subprocess.run([LQ16, 'run', '--params', params, '--state', p0, '--steps', str(2000 * T), '--algo', 'ntt', '--out', p1],
                             check=True, capture_output=True, text=True).stdout
        fin = read_state(p1).astype(np.float64) / 65535
    row.update(N=s0.shape[0], chain=[l.split()[1] for l in out.splitlines() if l.startswith('chain')][0],
               start_mass=round(float(s0.sum()) / 65535, 2), end_mass=round(float(fin.sum()), 2),
               survived=bool(fin.sum() > 1 and (fin > 0.01).mean() < 0.25))
    return row


def main():
    args = sys.argv[1:]
    jobs = []
    if args and args[0] == '--log':
        log, folder = json.load(open(args[1])), args[2]
        for ev in log['events']:
            if ev['type'] == 'collect':
                st = os.path.join(folder, ev['file'])
                jobs.append((st, st.replace('state-', 'params-').replace('.lq16', '.txt'), ev.get('kernel', 'cont')))
    else:
        kernel = 'cont'
        if '--kernel' in args:
            i = args.index('--kernel'); kernel = args[i + 1]; del args[i:i + 2]
        jobs = [(args[i], args[i + 1], kernel) for i in range(0, len(args), 2)]
    rows = []
    for st, pa, k in jobs:
        r = check(st, pa, k)
        rows.append(r)
        print(json.dumps(r), flush=True)
    for k in sorted({r['kernel'] for r in rows}):
        rs = [r for r in rows if r['kernel'] == k and r['survived'] is not None]
        if rs:
            dead = sum(not r['survived'] for r in rs)
            print(f'M7 kernel={k}: {len(rs)} candidates, {dead} did not survive, phantom rate {dead / len(rs):.0%}')


if __name__ == '__main__':
    main()
