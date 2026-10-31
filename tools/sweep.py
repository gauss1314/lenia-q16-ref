"""Random parameter sweep for self-found benchmark structures (float64 control).
Stays away from the known literature loci: Orbium (mu .15, sigma .016) and the
Paraptera cross-section (mu .30, sigma .0393-.0515), all beta=[1], using 2x the DD12
box (|dmu| <= .04, |dsigma| <= .008)."""
import json, sys
import numpy as np
from multiprocessing import Pool
sys.path.insert(0, '/home/user/lenia-q16-ref/tools')
from lenia_f64 import Lenia
N, R, T = 128, 13, 10
LIT = [(0.15, 0.016, '1/1')] + [(0.30, s / 10000, '1/1') for s in range(393, 516, 10)]
BETAS = ['1/1', '1/1', '1/2,1/1', '1/1,1/2', '1/1,1/3', '1/2,1/1,2/3', '1/3,1/1,1/2']

def near_lit(mu, sg, beta):
    return any(beta == b and abs(mu - m) <= 0.04 and abs(sg - s) <= 0.008 for m, s, b in LIT)

def try_params(job):
    idx, mu, sg, betas = job
    beta = [tuple(map(int, b.split('/'))) for b in betas.split(',')]
    L = Lenia(N, R, T, mu, sg, beta)
    yy, xx = np.mgrid[0:N, 0:N]
    out = []
    for seed in range(24):
        rng = np.random.default_rng(idx * 1000 + seed)
        amp = rng.uniform(0.3, 1.0); s = int(rng.integers(16, 48)); kind = seed % 3
        if kind == 0:
            A = np.zeros((N, N)); A[64 - s // 2:64 - s // 2 + s, 64 - s // 2:64 - s // 2 + s] = rng.random((s, s)) * amp
        elif kind == 1:
            w = s / 2.5; A = amp * np.exp(-((yy - 64) ** 2 + (xx - 64 - w / 3) ** 2) / (2 * w * w)) * (0.6 + 0.4 * rng.random((N, N)))
        else:
            rr = s / 2; A = amp * np.exp(-(np.sqrt((yy - 64) ** 2 + (xx - 64) ** 2) - rr) ** 2 / (2 * (rr / 4 + 1) ** 2)) * (0.5 + 0.5 * rng.random((N, N)))
        A = np.clip(A, 0, 1)
        ms = []
        for k in range(2000):
            A = L.step(A)
            if k >= 1000 and k % 25 == 0:
                ms.append(A.sum())
            if k == 300 and (A.sum() < 1 or (A > 0.01).mean() > 0.5):
                break
        if len(ms) < 10 or min(ms) < 1:
            continue
        var = (max(ms) - min(ms)) / np.mean(ms); nz = int((A > 0.01).sum())
        if var < 0.15 and nz < 3000:
            name = f'S{idx:02d}_{seed}'
            np.save(name + '.npy', A)
            out.append(dict(name=name, mu=round(mu, 4), sigma=round(sg, 4), beta=betas, seed=seed, kind=kind,
                            mass=round(float(np.mean(ms)), 1), var=round(float(var), 3), cells=nz))
            break  # one per parameter set
    return out

if __name__ == '__main__':
    rng = np.random.default_rng(20261031)
    jobs = []
    while len(jobs) < 40:
        mu = float(np.round(rng.uniform(0.10, 0.36), 4)); sg = float(np.round(mu * rng.uniform(0.07, 0.20), 4)); b = BETAS[rng.integers(len(BETAS))]
        if not near_lit(mu, sg, b):
            jobs.append((len(jobs), mu, sg, b))
    with Pool(4) as p:
        for res in p.imap_unordered(try_params, jobs):
            for r in res:
                print(json.dumps(r), flush=True)
    print('DONE', flush=True)
