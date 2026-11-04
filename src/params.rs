//! Rule parameters and validation (spec §3, §4).

use crate::intmath::gcd;

pub const SPEC_ID: &str = "lenia-q16/0.2-draft";
pub const LEGAL_N: [u32; 3] = [128, 256, 512];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Params {
    pub kf: u16,
    pub gf: u16,
    pub r: u16,
    pub t: u16,
    pub mu_q: u16,
    pub sigma_q: u16,
    /// β as (p, q) pairs, inner ring first.
    pub beta: Vec<(u16, u16)>,
}

impl Params {
    /// Checks every rule in spec §3; returns a reason on failure.
    pub fn validate(&self) -> Result<(), String> {
        if self.kf != 1 {
            return Err(format!("kf={} not defined", self.kf));
        }
        if self.gf != 1 {
            return Err(format!("gf={} not defined", self.gf));
        }
        if !(5..=50).contains(&self.r) {
            return Err(format!("R={} out of candidate range 5–50", self.r));
        }
        if !(5..=40).contains(&self.t) {
            return Err(format!("T={} out of candidate range 5–40", self.t));
        }
        if !(1..=9999).contains(&self.mu_q) {
            return Err("mu_q out of range".into());
        }
        if !(1..=9999).contains(&self.sigma_q) {
            return Err("sigma_q out of range".into());
        }
        let b = self.beta.len();
        if !(1..=4).contains(&b) {
            return Err(format!("B={} out of range 1–4", b));
        }
        let mut has_one = false;
        for &(p, q) in &self.beta {
            if !(1..=12).contains(&q) || p > q {
                return Err(format!("beta term {}/{} out of range", p, q));
            }
            if gcd(p as u64, q as u64) != 1 {
                return Err(format!("beta term {}/{} not reduced", p, q));
            }
            if p == q {
                has_one = true;
            }
        }
        if !has_one {
            return Err("beta must contain 1/1".into());
        }
        Ok(())
    }

    /// Parses "R=13 T=10 mu=1500 sigma=150 beta=1/1,1/2" (μ, σ in units of 1e-4).
    pub fn parse(s: &str) -> Result<Params, String> {
        let mut p = Params { kf: 1, gf: 1, r: 0, t: 0, mu_q: 0, sigma_q: 0, beta: vec![] };
        for tok in s.split_whitespace() {
            let (k, v) = tok.split_once('=').ok_or(format!("bad token {tok}"))?;
            let num = |v: &str| v.parse::<u16>().map_err(|e| format!("{k}: {e}"));
            match k {
                "kf" => p.kf = num(v)?,
                "gf" => p.gf = num(v)?,
                "R" => p.r = num(v)?,
                "T" => p.t = num(v)?,
                "mu" => p.mu_q = num(v)?,
                "sigma" => p.sigma_q = num(v)?,
                "beta" => {
                    for term in v.split(',') {
                        let (a, b) = term.split_once('/').ok_or("beta term needs p/q")?;
                        p.beta.push((num(a)?, num(b)?));
                    }
                }
                _ => return Err(format!("unknown key {k}")),
            }
        }
        p.validate()?;
        Ok(p)
    }
}

pub fn validate_n(n: u32, r: u16) -> Result<(), String> {
    if !LEGAL_N.contains(&n) {
        return Err(format!("N={} not a legal size", n));
    }
    if n < 2 * r as u32 + 1 {
        return Err("N < 2R+1".into());
    }
    Ok(())
}
