//! Serialisation (spec §8.1–§8.4).

use crate::kernel::Kernel;
use crate::params::{Params, SPEC_ID};

pub fn spec_id_field() -> [u8; 32] {
    let mut f = [0u8; 32];
    let b = SPEC_ID.as_bytes();
    f[..b.len()].copy_from_slice(b);
    f
}

fn header(tag: &[u8; 8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(tag);
    v.extend_from_slice(&spec_id_field());
    v
}

/// 72-byte parameter record (§8.2).
pub fn params_record(p: &Params) -> Vec<u8> {
    let mut v = header(b"LQ16PARM");
    for x in [p.kf, p.gf, p.r, p.t, p.mu_q, p.sigma_q, p.beta.len() as u16, 0] {
        v.extend_from_slice(&x.to_le_bytes());
    }
    for i in 0..4 {
        let (pp, qq) = p.beta.get(i).copied().unwrap_or((0, 0));
        v.extend_from_slice(&pp.to_le_bytes());
        v.extend_from_slice(&qq.to_le_bytes());
    }
    assert_eq!(v.len(), 72);
    v
}

pub fn kernel_record(k: &Kernel) -> Vec<u8> {
    let mut v = header(b"LQ16KERN");
    v.extend_from_slice(&(k.r as u32).to_le_bytes());
    for w in &k.weights {
        v.extend_from_slice(&w.to_le_bytes());
    }
    v
}

pub fn lut_record(lut: &[i32]) -> Vec<u8> {
    assert_eq!(lut.len(), 65536);
    let mut v = header(b"LQ16GLUT");
    for x in lut {
        v.extend_from_slice(&x.to_le_bytes());
    }
    assert_eq!(v.len(), 262184);
    v
}

pub fn state_record(n: u32, state: &[u16]) -> Vec<u8> {
    assert_eq!(state.len() as u64, n as u64 * n as u64);
    let mut v = header(b"LQ16STAT");
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&n.to_le_bytes());
    v.reserve(state.len() * 2);
    for a in state {
        v.extend_from_slice(&a.to_le_bytes());
    }
    v
}

/// Parses a state record, checking tag, spec id and N (§3, §8.4).
pub fn parse_state_record(bytes: &[u8]) -> Result<(u32, Vec<u16>), String> {
    if bytes.len() < 48 || &bytes[0..8] != b"LQ16STAT" {
        return Err("not a state record".into());
    }
    if bytes[8..40] != spec_id_field() {
        return Err("spec id differs from this implementation".into());
    }
    let w = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
    let h = u32::from_le_bytes(bytes[44..48].try_into().unwrap());
    if w != h {
        return Err("width != height".into());
    }
    let expect = 48 + 2 * (w as u64) * (w as u64);
    if bytes.len() as u64 != expect {
        return Err("state record length mismatch".into());
    }
    let state = bytes[48..].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Ok((w, state))
}
