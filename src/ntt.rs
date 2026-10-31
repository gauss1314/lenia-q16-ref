//! Exact potential U via number-theoretic transforms (spec §7.2 method 2).
//!
//! Two primes and the Chinese remainder theorem, 64-bit arithmetic only
//! (D-023, leaning option B). The true per-cell sum Σ w·A lies in
//! 0..2^32 − 2^16, below M1·M2, so CRT recovers it exactly. The correlation
//! in §7.1 is a convolution with the flipped kernel w'(e) = w(−e).
//! Moduli and this algorithm are implementation choices, not spec semantics.

use crate::kernel::Kernel;

const M1: u64 = 998_244_353; // 119·2^23 + 1, primitive root 3
const G1: u64 = 3;
const M2: u64 = 754_974_721; // 45·2^24 + 1, primitive root 11
const G2: u64 = 11;

fn pow_mod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u64;
    b %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % m;
        }
        b = b * b % m;
        e >>= 1;
    }
    r
}

/// In-place iterative radix-2 NTT of length n (power of two) over Z/m.
fn ntt<const M: u64, const G: u64>(a: &mut [u64], invert: bool) {
    let (m, g) = (M, G);
    let n = a.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let mut w = pow_mod(g, (m - 1) / len as u64, m);
        if invert {
            w = pow_mod(w, m - 2, m);
        }
        for start in (0..n).step_by(len) {
            let mut wn = 1u64;
            for k in 0..len / 2 {
                let u = a[start + k];
                let v = a[start + k + len / 2] * wn % M;
                a[start + k] = if u + v >= m { u + v - m } else { u + v };
                a[start + k + len / 2] = if u >= v { u - v } else { u + m - v };
                wn = wn * w % M;
            }
        }
        len <<= 1;
    }
    if invert {
        let inv_n = pow_mod(n as u64, m - 2, m);
        for x in a.iter_mut() {
            *x = *x * inv_n % M;
        }
    }
}

/// 2-D transform of an n×n row-major array.
fn ntt2<const M: u64, const G: u64>(a: &mut [u64], n: usize, invert: bool) {
    for row in a.chunks_mut(n) {
        ntt::<M, G>(row, invert);
    }
    let mut col = vec![0u64; n];
    for x in 0..n {
        for y in 0..n {
            col[y] = a[y * n + x];
        }
        ntt::<M, G>(&mut col, invert);
        for y in 0..n {
            a[y * n + x] = col[y];
        }
    }
}

pub struct NttPlan {
    n: usize,
    k1: Vec<u64>,
    k2: Vec<u64>,
    inv_m1_mod_m2: u64,
}

impl NttPlan {
    pub fn new(kernel: &Kernel, n: u32) -> NttPlan {
        let n = n as usize;
        let mut base = vec![0u64; n * n];
        for (dx, dy, w) in kernel.taps() {
            // flipped kernel w'(e) = w(−e): tap at offset d goes to position −d mod n
            let x = (-(dx as i64)).rem_euclid(n as i64) as usize;
            let y = (-(dy as i64)).rem_euclid(n as i64) as usize;
            base[y * n + x] = w as u64;
        }
        let mut k1 = base.clone();
        let mut k2 = base;
        ntt2::<M1, G1>(&mut k1, n, false);
        ntt2::<M2, G2>(&mut k2, n, false);
        NttPlan { n, k1, k2, inv_m1_mod_m2: pow_mod(M1 % M2, M2 - 2, M2) }
    }

    /// Returns U(x, y) = (Σ w·A) >> 16 for every cell, exactly.
    pub fn potential(&self, a: &[u16]) -> Vec<u16> {
        let n = self.n;
        let mut a1: Vec<u64> = a.iter().map(|&v| v as u64).collect();
        let mut a2 = a1.clone();
        ntt2::<M1, G1>(&mut a1, n, false);
        ntt2::<M2, G2>(&mut a2, n, false);
        for i in 0..n * n {
            a1[i] = a1[i] * self.k1[i] % M1;
            a2[i] = a2[i] * self.k2[i] % M2;
        }
        ntt2::<M1, G1>(&mut a1, n, true);
        ntt2::<M2, G2>(&mut a2, n, true);
        a1.iter()
            .zip(a2.iter())
            .map(|(&r1, &r2)| {
                // x = r1 + M1·t, t = (r2 − r1)·M1⁻¹ mod M2; x < M1·M2 < 2^60
                let t = ((r2 + M2 - r1 % M2) % M2) * self.inv_m1_mod_m2 % M2;
                let x = r1 + M1 * t;
                assert!(x < (1u64 << 32), "sum out of range: NTT/CRT error");
                (x >> 16) as u16
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::params::Params;
    use crate::world::Universe;

    fn soup(n: u32, seed: u64) -> Vec<u16> {
        // simple LCG soup for the test only
        let mut s = seed;
        (0..n * n)
            .map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (s >> 48) as u16
            })
            .collect()
    }

    #[test]
    fn ntt_matches_direct_sum() {
        for (r, beta) in [(5u16, vec![(1u16, 1u16)]), (13, vec![(1, 1)]), (31, vec![(1, 2), (1, 1), (2, 3)])] {
            let p = Params { kf: 1, gf: 1, r, t: 10, mu_q: 2700, sigma_q: 400, beta };
            let direct = Universe::new(p.clone(), 128).unwrap();
            let mut fast = Universe::new(p, 128).unwrap();
            fast.use_ntt();
            // full-range state, including 65535 everywhere, exercises the upper bound
            for a in [soup(128, 7), vec![65535u16; 128 * 128]] {
                assert_eq!(direct.step(&a, 2), fast.step(&a, 1), "R={r}");
            }
        }
    }
}
