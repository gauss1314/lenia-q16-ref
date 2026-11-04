//! Kernel generation (spec §5).

use crate::expneg::{ExpNeg, ONE};
use crate::intmath::{isqrt, lcm, rne};
use crate::params::Params;
use std::collections::BTreeMap;

/// Kernel weight total 2^KERNEL_BITS (draft 0.2: 20, D-028; draft 0.1 used 16).
pub const KERNEL_BITS: u32 = 20;
pub const W: i128 = 1 << KERNEL_BITS;
pub const S: i128 = 1 << 16;

pub struct Kernel {
    pub r: u16,
    /// (2R+1)² weights, row-major: j = (dy+R)(2R+1) + (dx+R).
    pub weights: Vec<u32>,
    /// Diagnostics from §5.4.
    pub h_total: i128,
    pub leftover_d: i128,
}

impl Kernel {
    pub fn side(&self) -> i32 {
        2 * self.r as i32 + 1
    }

    pub fn at(&self, dx: i32, dy: i32) -> u32 {
        let r = self.r as i32;
        let side = self.side();
        self.weights[((dy + r) * side + (dx + r)) as usize]
    }

    /// Non-zero taps as (dx, dy, w), row-major order.
    pub fn taps(&self) -> Vec<(i32, i32, u32)> {
        let r = self.r as i32;
        let mut out = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let w = self.at(dx, dy);
                if w != 0 {
                    out.push((dx, dy, w));
                }
            }
        }
        out
    }
}

pub fn generate(p: &Params, ex: &ExpNeg) -> Kernel {
    let r = p.r as i128;
    let b = p.beta.len() as i128;
    let side = 2 * r + 1;
    let l = p.beta.iter().fold(1u64, |acc, &(_, q)| lcm(acc, q as u64)) as i128;

    // h_j for every cell (0 outside the domain), and d² per cell.
    let mut h = vec![0i128; (side * side) as usize];
    let mut d2s = vec![-1i128; (side * side) as usize];
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 >= r * r {
                continue;
            }
            let j = ((dy + r) * side + (dx + r)) as usize;
            d2s[j] = d2;
            let num = d2 * b * b * S * S;
            let rho = isqrt((num / (r * r)) as u64) as i128; // num ≥ 0 so / is floor
            let i = (rho >> 16) as usize;
            let f = rho % S;
            let k = if f == 0 {
                0
            } else {
                let y = rne(1i128 << 62, f * (S - f)) - (1i128 << 32);
                ex.eval(y)
            };
            debug_assert!((0..=ONE).contains(&k));
            let (pi, qi) = p.beta[i];
            h[j] = k * pi as i128 * (l / qi as i128);
        }
    }

    let h_total: i128 = h.iter().sum();
    assert!(h_total > 0, "H = 0: illegal rule point");

    let mut a = vec![0i128; h.len()];
    let mut rem = vec![0i128; h.len()];
    for j in 0..h.len() {
        a[j] = W * h[j] / h_total; // both non-negative: floor
        rem[j] = W * h[j] - a[j] * h_total;
    }
    let mut d = W - a.iter().sum::<i128>();

    // Classes by d² among domain cells with remainder > 0.
    let mut classes: BTreeMap<i128, (i128, Vec<usize>)> = BTreeMap::new();
    for j in 0..h.len() {
        if d2s[j] >= 0 && rem[j] > 0 {
            let e = classes.entry(d2s[j]).or_insert((rem[j], Vec::new()));
            assert_eq!(e.0, rem[j], "cells with equal d² must have equal remainder");
            e.1.push(j);
        }
    }
    let mut order: Vec<(i128, i128, Vec<usize>)> =
        classes.into_iter().map(|(d2, (rm, cells))| (rm, d2, cells)).collect();
    // r descending; ties by d² ascending.
    order.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    for (_, _, cells) in &order {
        let n = cells.len() as i128;
        if n <= d {
            for &j in cells {
                a[j] += 1;
            }
            d -= n;
        }
    }
    let leftover_d = d;
    let center = (r * side + r) as usize;
    a[center] += d;

    let weights: Vec<u32> = a.iter().map(|&x| u32::try_from(x).unwrap()).collect();
    debug_assert_eq!(weights.iter().map(|&w| w as u64).sum::<u64>(), 1 << KERNEL_BITS);
    Kernel { r: p.r, weights, h_total, leftover_d }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(r: u16, beta: &[(u16, u16)]) -> Params {
        Params { kf: 1, gf: 1, r, t: 10, mu_q: 1500, sigma_q: 150, beta: beta.to_vec() }
    }

    #[test]
    fn r14_single_ring_matches_spec_notes() {
        let ex = ExpNeg::new();
        let k = generate(&params(14, &[(1, 1)]), &ex);
        assert_eq!(k.h_total, 253767521820);
        // (max, w(9,0), w(3,6)) for each kernel total
        let (mx, w90, w36) = if KERNEL_BITS == 16 { (277, 195, 275) } else { (4437, 3109, 4406) };
        assert_eq!(*k.weights.iter().max().unwrap(), mx);
        assert_eq!(k.at(9, 0), w90);
        assert_eq!(k.at(3, 6), w36);
        assert_eq!(k.at(6, 3), w36);
    }

    #[test]
    fn kernels_sum_and_symmetric() {
        let ex = ExpNeg::new();
        let betas: [&[(u16, u16)]; 4] =
            [&[(1, 1)], &[(1, 2), (1, 1), (2, 3)], &[(1, 1), (1, 3)], &[(1, 4), (3, 4), (1, 1), (5, 12)]];
        for beta in betas {
            for r in 5..=50u16 {
                let k = generate(&params(r, beta), &ex);
                let sum: u64 = k.weights.iter().map(|&w| w as u64).sum();
                assert_eq!(sum, 1 << KERNEL_BITS);
                // 2^16: always 0 in this range. 2^20: a few units can be left for the centre
                // (spec §5.4 rule), which keeps the kernel symmetric; at most 4 over R = 5..50 and these rings.
                assert!(k.leftover_d >= 0 && k.leftover_d < 64, "R={r} leftover {}", k.leftover_d);
                let ri = r as i32;
                for dy in -ri..=ri {
                    for dx in -ri..=ri {
                        let w = k.at(dx, dy);
                        assert_eq!(w, k.at(-dx, dy));
                        assert_eq!(w, k.at(dx, -dy));
                        assert_eq!(w, k.at(dy, dx));
                    }
                }
            }
        }
    }
}
