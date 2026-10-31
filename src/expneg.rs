//! Fixed-point e^(−t) from spec §5.5.

use crate::intmath::rne;

pub const ONE: i128 = 1 << 30;

fn series(g: i128) -> i128 {
    debug_assert!((0..=ONE).contains(&g));
    let mut acc = ONE;
    let mut term = ONE;
    for m in 1..=16i128 {
        term = rne(term * g, m * ONE);
        if m % 2 == 1 {
            acc -= term;
        } else {
            acc += term;
        }
    }
    acc
}

/// ENEG[n] for n = 0..=21, computed exactly as the spec's procedure.
pub fn eneg_table() -> [i128; 22] {
    let e1 = series(ONE);
    let mut t = [0i128; 22];
    t[0] = ONE;
    for n in 1..22 {
        t[n] = rne(t[n - 1] * e1, ONE);
    }
    t
}

/// Precomputed context so ENEG is built once.
pub struct ExpNeg {
    eneg: [i128; 22],
}

impl Default for ExpNeg {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpNeg {
    pub fn new() -> Self {
        ExpNeg { eneg: eneg_table() }
    }

    /// expneg(y) ≈ e^(−y/ONE)·ONE for y ≥ 0.
    pub fn eval(&self, y: i128) -> i128 {
        assert!(y >= 0);
        if y >= 22 * ONE {
            return 0;
        }
        let n = (y >> 30) as usize;
        let g = y - (n as i128) * ONE;
        rne(series(g) * self.eneg[n], ONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eneg_matches_spec_table() {
        let expect: [i128; 22] = [
            1073741824, 395007541, 145315153, 53458457, 19666267, 7234815, 2661540, 979126,
            360200, 132510, 48748, 17933, 6597, 2427, 893, 329, 121, 45, 17, 6, 2, 1,
        ];
        assert_eq!(eneg_table(), expect);
    }

    #[test]
    fn expneg_check_values() {
        let e = ExpNeg::new();
        assert_eq!(e.eval(0), 1073741824);
        assert_eq!(e.eval(1 << 29), 651257338);
        assert_eq!(e.eval(ONE), 395007541);
        assert_eq!(e.eval(22 * ONE - 1), 0);
        assert_eq!(e.eval(22 * ONE), 0);
    }
}
