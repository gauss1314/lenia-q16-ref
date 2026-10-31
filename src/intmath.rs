//! Integer helpers from spec §2. Every function works on mathematical
//! integers with the bounds stated in the spec; no floating point anywhere.

/// Floor division ⌊a / b⌋ for b > 0 (rounds toward negative infinity).
pub fn floor_div(a: i128, b: i128) -> i128 {
    assert!(b > 0);
    a.div_euclid(b)
}

/// a mod b in 0..b for b > 0.
pub fn floor_mod(a: i128, b: i128) -> i128 {
    assert!(b > 0);
    a.rem_euclid(b)
}

/// Round half to even: rne(a, b) for b > 0 (spec §2).
pub fn rne(a: i128, b: i128) -> i128 {
    assert!(b > 0);
    let q = floor_div(a, b);
    let r = floor_mod(a, b);
    let twice = 2 * r;
    if twice < b {
        q
    } else if twice > b {
        q + 1
    } else if q % 2 == 0 {
        q
    } else {
        q + 1
    }
}

/// Integer square root: largest k with k² ≤ n, for 0 ≤ n < 2^48 (spec §2).
/// Bit-by-bit method, integer only.
pub fn isqrt(n: u64) -> u64 {
    assert!(n < (1u64 << 48), "isqrt input out of range");
    let mut rem = n;
    let mut root: u64 = 0;
    let mut bit: u64 = 1u64 << 46; // highest even power of two ≤ 2^47
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

pub fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

pub fn lcm(a: u64, b: u64) -> u64 {
    a / gcd(a, b) * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rne_spec_examples() {
        assert_eq!(rne(-3, 2), -2);
        assert_eq!(rne(-5, 2), -2);
        assert_eq!(rne(5, 2), 2);
        assert_eq!(rne(7, 2), 4);
        assert_eq!(rne(1, 3), 0);
        assert_eq!(rne(2, 3), 1);
    }

    #[test]
    fn isqrt_edges() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(3), 1);
        assert_eq!(isqrt(4), 2);
        let k: u64 = (1 << 24) - 1;
        assert_eq!(isqrt(k * k), k);
        assert_eq!(isqrt(k * k - 1), k - 1);
        assert_eq!(isqrt((1u64 << 48) - 1), k);
        for k in [2u64, 3, 1000, 65535, 123456] {
            assert_eq!(isqrt(k * k), k);
            assert_eq!(isqrt(k * k - 1), k - 1);
            assert_eq!(isqrt(k * k + 1), k);
        }
    }
}
