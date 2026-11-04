//! One update step by direct summation (spec §7.1, §7.2 method 1) and the
//! trajectory hash chain (§8.5).

use crate::expneg::ExpNeg;
use crate::kernel::{generate as gen_kernel, Kernel, KERNEL_BITS};
use crate::lut::generate as gen_lut;
use crate::ntt::NttPlan;
use crate::params::{validate_n, Params, SPEC_ID};
use crate::records::{kernel_record, lut_record, params_record, spec_id_field, state_record};
use crate::sha256::sha256;

pub const CHAIN_INTERVAL: u64 = 256;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Algo {
    Direct,
    Ntt,
}

pub struct Universe {
    pub algo: Algo,
    ntt: Option<NttPlan>,
    pub params: Params,
    pub n: u32,
    pub kernel: Kernel,
    pub lut: Vec<i32>,
    taps: Vec<(i32, i32, u64)>,
    pub h_params: [u8; 32],
    pub h_kernel: [u8; 32],
    pub h_lut: [u8; 32],
}

impl Universe {
    pub fn new(params: Params, n: u32) -> Result<Self, String> {
        params.validate()?;
        validate_n(n, params.r)?;
        let ex = ExpNeg::new();
        let kernel = gen_kernel(&params, &ex);
        let lut = gen_lut(&params, &ex);
        let taps = kernel.taps().into_iter().map(|(x, y, w)| (x, y, w as u64)).collect();
        let h_params = sha256(&params_record(&params));
        let h_kernel = sha256(&kernel_record(&kernel));
        let h_lut = sha256(&lut_record(&lut));
        Ok(Universe { algo: Algo::Direct, ntt: None, params, n, kernel, lut, taps, h_params, h_kernel, h_lut })
    }

    /// Computes rows [y0, y1) of the next state into `out`.
    fn step_rows(&self, a: &[u16], out: &mut [u16], y0: u32, y1: u32) {
        let n = self.n as i64;
        for y in y0..y1 {
            for x in 0..self.n {
                let mut acc: u64 = 0;
                for &(dx, dy, w) in &self.taps {
                    let xx = (x as i64 + dx as i64).rem_euclid(n);
                    let yy = (y as i64 + dy as i64).rem_euclid(n);
                    let v = a[(yy * n + xx) as usize] as u64;
                    acc = acc.checked_add(w * v).expect("u64 accumulator overflow");
                }
                let u = acc >> KERNEL_BITS;
                let next = a[(y as i64 * n + x as i64) as usize] as i32 + self.lut[u as usize];
                out[((y - y0) as i64 * n + x as i64) as usize] = next.clamp(0, 65535) as u16;
            }
        }
    }

    /// Switches to the NTT potential (spec §7.2 method 2); results must not change.
    pub fn use_ntt(&mut self) {
        self.ntt = Some(NttPlan::new(&self.kernel, self.n));
        self.algo = Algo::Ntt;
    }

    /// One synchronous step; `threads` only splits the work (result is identical).
    pub fn step(&self, a: &[u16], threads: u32) -> Vec<u16> {
        if let Some(plan) = &self.ntt {
            let u = plan.potential(a);
            return a
                .iter()
                .zip(u.iter())
                .map(|(&v, &uu)| (v as i32 + self.lut[uu as usize]).clamp(0, 65535) as u16)
                .collect();
        }
        let n = self.n;
        let mut out = vec![0u16; a.len()];
        let threads = threads.max(1).min(n);
        if threads == 1 {
            self.step_rows(a, &mut out, 0, n);
            return out;
        }
        let rows_per = (n + threads - 1) / threads;
        std::thread::scope(|s| {
            let mut rest: &mut [u16] = &mut out;
            let mut y0 = 0u32;
            while y0 < n {
                let y1 = (y0 + rows_per).min(n);
                let (chunk, tail) = rest.split_at_mut(((y1 - y0) * n) as usize);
                rest = tail;
                s.spawn(move || self.step_rows(a, chunk, y0, y1));
                y0 = y1;
            }
        });
        out
    }

    pub fn chain_head(&self, s0: &[u8; 32]) -> [u8; 32] {
        let mut v = Vec::new();
        v.extend_from_slice(b"LQ16CHN0");
        v.extend_from_slice(&spec_id_field());
        v.extend_from_slice(&self.h_params);
        v.extend_from_slice(&self.h_kernel);
        v.extend_from_slice(&self.h_lut);
        v.extend_from_slice(&(CHAIN_INTERVAL as u32).to_le_bytes());
        v.extend_from_slice(s0);
        sha256(&v)
    }

    /// Runs L steps from `state`, returning the final state, the chain and every
    /// segment point (k, S_k) including k = 0.
    pub fn run(&self, state: Vec<u16>, l: u64, threads: u32) -> RunResult {
        self.run_with(state, l, threads, |_, _| {})
    }

    /// Same as `run`, calling `observe(k, state)` after every step k (tools only).
    pub fn run_with(&self, state: Vec<u16>, l: u64, threads: u32, mut observe: impl FnMut(u64, &[u16])) -> RunResult {
        let s0 = sha256(&state_record(self.n, &state));
        let mut c = self.chain_head(&s0);
        let mut points = vec![(0u64, s0)];
        let mut a = state;
        for k in 1..=l {
            a = self.step(&a, threads);
            observe(k, &a);
            if k % CHAIN_INTERVAL == 0 || k == l {
                let sk = sha256(&state_record(self.n, &a));
                let mut v = Vec::new();
                v.extend_from_slice(b"LQ16LINK");
                v.extend_from_slice(&c);
                v.extend_from_slice(&k.to_le_bytes());
                v.extend_from_slice(&sk);
                c = sha256(&v);
                points.push((k, sk));
            }
        }
        RunResult { final_state: a, chain: c, points, spec: SPEC_ID }
    }
}

pub struct RunResult {
    pub final_state: Vec<u16>,
    pub chain: [u8; 32],
    pub points: Vec<(u64, [u8; 32])>,
    pub spec: &'static str,
}
