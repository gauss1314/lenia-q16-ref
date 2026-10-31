//! Minimal C-ABI wrapper so a browser can run the integer reference
//! single-threaded (spec §7.3: no relaxed SIMD, no threads).
//!
//! Protocol: the host writes a parameter record and a state record into
//! memory obtained from `lq16_alloc`, calls `lq16_run`, then reads the
//! 64-byte hex chain and the 64-byte hex hash of the final state from the
//! buffer returned by `lq16_result_ptr`.

use lq16::records::{parse_params_record, parse_state_record, state_record};
use lq16::sha256::{hex, sha256};
use lq16::world::Universe;

static mut RESULT: [u8; 256] = [0; 256];

#[no_mangle]
pub extern "C" fn lq16_alloc(len: u32) -> *mut u8 {
    let mut v = vec![0u8; len as usize];
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Returns 0 on success, 1 on bad input (message in the result buffer).
#[no_mangle]
pub extern "C" fn lq16_run(pp: *const u8, plen: u32, sp: *const u8, slen: u32, steps: u32) -> u32 {
    let params = unsafe { std::slice::from_raw_parts(pp, plen as usize) };
    let state = unsafe { std::slice::from_raw_parts(sp, slen as usize) };
    let out = (|| -> Result<String, String> {
        let p = parse_params_record(params)?;
        let (n, s) = parse_state_record(state)?;
        let u = Universe::new(p, n)?;
        let r = u.run(s, steps as u64, 1);
        Ok(format!("{}{}", hex(&r.chain), hex(&sha256(&state_record(n, &r.final_state)))))
    })();
    let (code, text) = match out {
        Ok(t) => (0, t),
        Err(e) => (1, e),
    };
    unsafe {
        let buf = &mut *std::ptr::addr_of_mut!(RESULT);
        buf.fill(0);
        let b = text.as_bytes();
        let n = b.len().min(255);
        buf[..n].copy_from_slice(&b[..n]);
    }
    code
}

#[no_mangle]
pub extern "C" fn lq16_result_ptr() -> *const u8 {
    std::ptr::addr_of!(RESULT) as *const u8
}
