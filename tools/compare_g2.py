"""G2 behaviour comparison (Phase 0, not normative). Measurement report §1 table.

  python3 compare_g2.py int   <params> <state.lq16> [TUs]   integer reference vs float64
  python3 compare_g2.py noise <params> <state.lq16> [TUs] [seed]   float64 vs float64 with +-1 LSB
                                                            on every non-zero cell (seed default 1)

<params> is the lq16 string, e.g. "R=13 T=10 mu=1923 sigma=277 beta=1/2,1/1".
Both runs start from the same quantised snapshot and run TUs time units (default 2000).
Statistics use the last half of the run (1000 TU at the default); morphology uses the
last 100 TU. Prints one JSON line: both sides' statistics and every item's difference.
"""
import json, os, subprocess, sys, tempfile
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lenia_f64 import Lenia, read_state

LQ16 = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'target', 'release', 'lq16')
ALIVE_MIN_MASS, BLOWUP_FRAC, MASK = 1.0, 0.25, 0.1
SPEED_FLOOR = 0.002  # R per TU; below this the speed is centroid jitter and is not compared
ANGLES = np.arange(0, 360, 5)
HARMONICS = np.arange(1, 13)
PSI = np.radians(np.arange(-180, 180, 0.5))


def rotation_steps(h):
    """Rotation (deg) between consecutive frames: the angle maximising the circular
    cross-correlation of the angular profiles, from harmonics m = 1..12 about the centroid.
    A rotation by phi multiplies harmonic m by exp(-i m phi)."""
    cross = h[1:] * np.conj(h[:-1])  # (frames-1, M)
    score = np.real(cross @ np.exp(1j * HARMONICS[:, None] * PSI[None, :]))
    return np.degrees(PSI[np.argmax(score, axis=1)])


def parse(params):
    d = dict(kv.split('=') for kv in params.split())
    beta = [tuple(map(int, b.split('/'))) for b in d['beta'].split(',')]
    return int(d['R']), int(d['T']), int(d['mu']) / 1e4, int(d['sigma']) / 1e4, beta


class Stats:
    """Per-TU statistics; keeps centred crops of the last `keep` frames."""

    def __init__(self, n, R, tus, keep=100):
        self.n, self.R, self.tus, self.keep = n, R, tus, keep
        self.c = min(n, max(64, 8 * R)) // 2 * 2
        ang = 2 * np.pi * np.arange(n) / n
        self.e = np.exp(1j * ang)
        self.mass, self.cen, self.harm, self.cover, self.crops = [], [], [], [], []

    def add(self, k, A):
        m = A.sum()
        self.mass.append(m)
        self.cover.append((A > 0.01).mean())
        if m <= 0:
            self.cen.append((0.0, 0.0)); self.harm.append(np.zeros(len(HARMONICS), complex))
            if k >= self.tus - self.keep:
                self.crops.append(np.zeros((self.c, self.c), np.float32))
            return
        cy = np.angle((A.sum(1) * self.e).sum()) * self.n / (2 * np.pi)
        cx = np.angle((A.sum(0) * self.e).sum()) * self.n / (2 * np.pi)
        self.cen.append((cy, cx))
        B = np.roll(A, (self.n // 2 - int(round(cy)), self.n // 2 - int(round(cx))), axis=(0, 1))
        h = self.c // 2
        C = B[self.n // 2 - h:self.n // 2 + h, self.n // 2 - h:self.n // 2 + h]
        yy, xx = np.mgrid[-h:h, -h:h]
        w = C / max(C.sum(), 1e-12)
        my, mx = (w * yy).sum(), (w * xx).sum()
        th = np.arctan2(yy - my, xx - mx)
        self.harm.append((C[None] * np.exp(-1j * HARMONICS[:, None, None] * th[None])).sum((1, 2)))
        if k >= self.tus - self.keep:
            # sub-pixel centring: shift the whole frame by the fractional remainder (Fourier shift)
            fy, fx = cy - round(cy), cx - round(cx)
            ky = np.fft.fftfreq(self.n)[:, None]; kx = np.fft.fftfreq(self.n)[None, :]
            Bs = np.real(np.fft.ifft2(np.fft.fft2(B) * np.exp(2j * np.pi * (ky * fy + kx * fx))))
            self.crops.append(Bs[self.n // 2 - h:self.n // 2 + h, self.n // 2 - h:self.n // 2 + h].astype(np.float32))

    def summary(self):
        tail = slice(self.tus // 2, None)
        m = np.array(self.mass)
        alive = bool(m[-1] > ALIVE_MIN_MASS and self.cover[-1] < BLOWUP_FRAC)
        cen = np.array(self.cen)
        d = cen[1:] - cen[:-1]
        d = (d + self.n / 2) % self.n - self.n / 2
        d = d[self.tus // 2:]
        v = d.mean(0)
        speed = float(np.hypot(d[:, 0], d[:, 1]).mean() / self.R)  # R per TU
        direction = float(np.degrees(np.arctan2(v[0], v[1])))
        omega = float(np.median(rotation_steps(np.array(self.harm)[self.tus // 2:])))  # deg per TU; median: symmetric shapes alias
        mt = m[tail]
        x = mt - mt.mean()
        spec = np.abs(np.fft.rfft(x * np.hanning(len(x))))
        spec[0] = 0
        f = int(np.argmax(spec))
        freq = float(f / len(x)) if spec[f] > 0 else 0.0  # cycles per TU
        cv = float(mt.std() / mt.mean()) if mt.mean() > 0 else 0.0
        if not alive:
            cls = 'dead' if m[-1] <= ALIVE_MIN_MASS else 'unbounded'
        elif np.hypot(*v) / self.R > 0.002:
            cls = 'mover'
        elif abs(omega) > 0.5:
            cls = 'rotator'
        elif cv > 0.01:
            cls = 'oscillator'
        else:
            cls = 'stationary'
        return dict(alive=alive, cls=cls, mass=float(mt.mean()), mass_cv=cv, speed=speed,
                    direction=direction, omega=omega, freq=freq)


def rotate(C, deg):
    """Bilinear rotation of a centred crop about its centre."""
    c = C.shape[0]; h = c / 2
    yy, xx = np.mgrid[0:c, 0:c] + 0.5 - h
    a = np.radians(deg)
    sy = np.cos(a) * yy - np.sin(a) * xx + h - 0.5
    sx = np.sin(a) * yy + np.cos(a) * xx + h - 0.5
    y0 = np.floor(sy).astype(int); x0 = np.floor(sx).astype(int)
    fy = sy - y0; fx = sx - x0
    def g(y, x):
        ok = (y >= 0) & (y < c) & (x >= 0) & (x < c)
        return np.where(ok, C[np.clip(y, 0, c - 1), np.clip(x, 0, c - 1)], 0)
    return (g(y0, x0) * (1 - fy) * (1 - fx) + g(y0 + 1, x0) * fy * (1 - fx)
            + g(y0, x0 + 1) * (1 - fy) * fx + g(y0 + 1, x0 + 1) * fy * fx)


def morph_iou(ref_crops, other_crops):
    """For each ref frame, the best IoU (masks at MASK) over the other run's frames and
    rotations in 5-degree steps; returns the median over ref frames."""
    if not ref_crops or not other_crops:
        return 0.0
    c = ref_crops[0].shape[0]
    f = -(-c // 128)  # large R: block-average crops down to <= 128 px so the match fits in memory
    if f > 1:
        m = c // f * f
        down = lambda M: M[:m, :m].reshape(m // f, f, m // f, f).mean((1, 3))
        ref_crops, other_crops = [down(M) for M in ref_crops], [down(M) for M in other_crops]
    G = np.array([(rotate(M, d) > MASK).ravel() for d in ANGLES for M in other_crops], np.float32)
    F = np.array([(M > MASK).ravel() for M in ref_crops], np.float32)
    inter = F @ G.T
    union = F.sum(1)[:, None] + G.sum(1)[None, :] - inter
    iou = np.where(union > 0, inter / np.maximum(union, 1), 1.0)
    return float(np.median(iou.max(1)))


def run_f64(params, A, tus):
    R, T, mu, sg, beta = parse(params)
    n = A.shape[0]
    L = Lenia(n, R, T, mu, sg, beta)
    st = Stats(n, R, tus)
    for k in range(tus):
        for _ in range(T):
            A = L.step(A)
        st.add(k, A)
    return st


def run_int(params, state_path, tus):
    R, T, *_ = parse(params)
    n = read_state(state_path).shape[0]
    st = Stats(n, R, tus)
    with tempfile.TemporaryDirectory() as tmp:
        dump = os.path.join(tmp, 'frames.u16')
        subprocess.run([LQ16, 'run', '--params', params, '--state', state_path, '--steps', str(tus * T),
                        '--algo', 'ntt', '--dump', dump, '--every', str(T)], check=True, capture_output=True)
        with open(dump, 'rb') as fh:
            for k in range(tus):
                A = np.frombuffer(fh.read(2 * n * n), '<u2').reshape(n, n) / 65535.0
                st.add(k, A)
    return st


def diff(a, b, ra, rb):
    rel = lambda x, y: abs(x - y) / max(abs(x), abs(y), 1e-12)
    dd = abs((a['direction'] - b['direction'] + 180) % 360 - 180)
    return dict(
        alive_same=a['alive'] == b['alive'], cls_same=a['cls'] == b['cls'],
        speed_rel=rel(a['speed'], b['speed']) if max(a['speed'], b['speed']) > SPEED_FLOOR else None,
        direction_deg=dd if a['cls'] == 'mover' and b['cls'] == 'mover' else None,
        omega_rel=rel(a['omega'], b['omega']) if 'rotator' in (a['cls'], b['cls']) else None,
        freq_rel=rel(a['freq'], b['freq']) if a['mass_cv'] > 0.01 or b['mass_cv'] > 0.01 else None,
        mass_rel=rel(a['mass'], b['mass']),
        morph_iou=morph_iou(rb.crops, ra.crops))


def main():
    mode, params, state_path = sys.argv[1:4]
    tus = int(sys.argv[4]) if len(sys.argv) > 4 else 2000
    q = read_state(state_path)
    A0 = q.astype(np.float64) / 65535
    ref = run_f64(params, A0, tus)
    if mode == 'int':
        other = run_int(params, state_path, tus)
    elif mode == 'noise':
        rng = np.random.default_rng(int(sys.argv[5]) if len(sys.argv) > 5 else 1)
        p = q.astype(np.int64) + rng.choice([-1, 1], size=q.shape) * (q > 0)
        other = run_f64(params, np.clip(p, 0, 65535) / 65535, tus)
    else:
        sys.exit('mode is int or noise')
    a, b = other.summary(), ref.summary()
    out = dict(mode=mode, seed=int(sys.argv[5]) if mode == 'noise' and len(sys.argv) > 5 else 1, params=params, state=os.path.basename(state_path), tus=tus,
               other=a, f64=b, diff=diff(a, b, other, ref))
    print(json.dumps(out, default=float), flush=True)


if __name__ == '__main__':
    main()
