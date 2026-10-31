//! Growth lookup table (spec §6).

use crate::expneg::{ExpNeg, ONE};
use crate::intmath::rne;
use crate::params::Params;

pub fn generate(p: &Params, ex: &ExpNeg) -> Vec<i32> {
    let mu = p.mu_q as i128;
    let sigma = p.sigma_q as i128;
    let t = p.t as i128;
    let den = 65535 * sigma;
    let mut lut = Vec::with_capacity(65536);
    for u in 0..65536i128 {
        let delta = 10000 * u - 65535 * mu;
        let ad = delta.abs();
        let e = if ad >= 7 * den {
            0
        } else {
            let v = rne(ad << 28, den);
            let y = rne(v * v, 1 << 27);
            ex.eval(y)
        };
        let g = 2 * e - ONE;
        lut.push(i32::try_from(rne(65535 * g, t * ONE)).unwrap());
    }
    lut
}
