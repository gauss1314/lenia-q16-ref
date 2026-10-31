"""Benchmark pipeline for one cultured structure (Phase 0, not normative).

python3 pipeline.py <label> <npy> <mu_q> <sigma_q> <betas> [R] [T]

1. Recentre the float64 structure, place it in 256x256, quantise to u16 (round half even).
2. Integer reference: run 200 TU (2000 steps at T=10) with the NTT path, record chain.
3. float64 control: run the same quantised start for the same steps.
4. Compare behaviour over the last 100 TU: mean mass, mass variability, mean speed.
5. Refinement (AS11 direction only): 2R (state upsampled 2x by pixel replication then
   relaxed in float64 at 2R for 50 TU), and 4T. Survival = mass > 0 at the end and no
   blow-up (< 25% of the torus above 0.01).
Prints one JSON line. Writes state records next to the npy.
"""
import json, subprocess, sys
import numpy as np
sys.path.insert(0, '/home/user/lenia-q16-ref/tools')
from lenia_f64 import Lenia, quantize, state_record, read_state

LQ16 = '/home/user/lenia-q16-ref/target/release/lq16'
label, npy, mu_q, sigma_q, betas = sys.argv[1:6]
R = int(sys.argv[6]) if len(sys.argv) > 6 else 13
T = int(sys.argv[7]) if len(sys.argv) > 7 else 10
mu, sigma = int(mu_q) / 1e4, int(sigma_q) / 1e4
beta = [tuple(map(int, b.split('/'))) for b in betas.split(',')]


def place(A, n=256, crop=96):
    N = A.shape[0]
    y, x = np.unravel_index(np.argmax(A), A.shape)
    A = np.roll(A, (N // 2 - y, N // 2 - x), axis=(0, 1))
    c = min(crop, N) // 2
    B = np.zeros((n, n))
    B[n // 2 - c:n // 2 + c, n // 2 - c:n // 2 + c] = A[N // 2 - c:N // 2 + c, N // 2 - c:N // 2 + c]
    return B


def lq16_run(params, state_path, steps, out=None):
    cmd = [LQ16, 'run', '--params', params, '--state', state_path, '--steps', str(steps), '--algo', 'ntt']
    if out:
        cmd += ['--out', out]
    txt = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
    return {l.split()[0]: l.split()[1] for l in txt.splitlines() if len(l.split()) > 1}


def track(A_seq, n):
    ang = 2 * np.pi * np.arange(n) / n
    ms, cs = [], []
    for A in A_seq:
        ms.append(A.sum())
        cs.append([np.angle((A.sum(1) * np.exp(1j * ang)).sum()), np.angle((A.sum(0) * np.exp(1j * ang)).sum())])
    cs = np.array(cs)
    d = np.angle(np.exp(1j * np.diff(cs, axis=0))) * n / (2 * np.pi)
    return np.array(ms), np.hypot(d[:, 0], d[:, 1])


def params_str(r, t):
    return f'R={r} T={t} mu={mu_q} sigma={sigma_q} beta={betas}'


out = {'label': label, 'params': params_str(R, T)}
A0 = place(np.load(npy))
q0 = quantize(A0)
s0 = npy.replace('.npy', '_s0.lq16')
open(s0, 'wb').write(state_record(q0))

# integer reference, sampled every T steps over 200 TU via chained runs of T steps
steps_tu, tus = T, 200
ints, cur = [], s0
tmp = npy.replace('.npy', '_tmp.lq16')
res = lq16_run(params_str(R, T), s0, steps_tu * tus, out=npy.replace('.npy', '_int_end.lq16'))
out['chain'] = res['chain']
# sampled integer trajectory (separate, cheaper: 1 TU per call is slow; sample every 2 TU)
cur = s0
for k in range(tus // 2):
    lq16_run(params_str(R, T), cur, 2 * T, out=tmp)
    ints.append(read_state(tmp).astype(np.float64) / 65535)
    cur = tmp
# float64 control from the same quantised start
L = Lenia(256, R, T, mu, sigma, beta)
A = q0.astype(np.float64) / 65535
flts = []
for k in range(tus // 2):
    for _ in range(2 * T):
        A = L.step(A)
    flts.append(A.copy())
for name, seq in (('int', ints), ('f64', flts)):
    m, v = track(seq[len(seq) // 2:], 256)
    out[name] = dict(mass=round(float(m.mean()), 3), mass_cv=round(float(m.std() / m.mean()), 4) if m.mean() > 0 else None,
                     speed_per_tu=round(float(v.mean() / 2), 4), alive=bool(m[-1] > 1))

# refinement: 2R and 4T (integer reference), 100 TU each
def survive(state_rec, r, t, tus=100):
    end = npy.replace('.npy', f'_ref_R{r}_T{t}.lq16')
    lq16_run(params_str(r, t), state_rec, t * tus, out=end)
    S = read_state(end).astype(np.float64) / 65535
    return bool(S.sum() > 1 and (S > 0.01).mean() < 0.25), round(float(S.sum()), 2)

if 2 * R <= 50:
    # upsample 2x, relax in float64 at 2R, then quantise
    A2 = np.kron(A0[64:192, 64:192], np.ones((2, 2)))
    L2 = Lenia(256, 2 * R, T, mu, sigma, beta)
    for _ in range(50 * T):
        A2 = L2.step(A2)
    s2 = npy.replace('.npy', '_s0_2R.lq16'); open(s2, 'wb').write(state_record(quantize(A2)))
    out['refine_2R'] = survive(s2, 2 * R, T)
else:
    out['refine_2R'] = 'untestable (R>50)'
out['refine_4T'] = survive(s0, R, 4 * T) if 4 * T <= 40 else 'untestable (T>40)'
print(json.dumps(out), flush=True)
