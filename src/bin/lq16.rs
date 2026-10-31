//! Command-line front end.
//!
//!   lq16 info  "<params>" [out_dir]   (out_dir: write params/kernel/lut records)
//!   lq16 run   --params "<params>" --state <state.lq16> --steps L
//!              [--threads K] [--algo direct|ntt] [--out final.lq16] [--points points.tsv]
//!   lq16 soup  --out s.lq16 [--n 256] [--size 40] [--amp 65535] [--seed 1]
//!
//! <params> looks like "R=13 T=10 mu=1500 sigma=150 beta=1/1".

use lq16::params::Params;
use lq16::records::{parse_state_record, state_record};
use lq16::sha256::{hex, sha256};
use lq16::world::Universe;
use std::collections::HashMap;
use std::process::exit;

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    exit(2)
}

fn flags(args: &[String]) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let mut i = 0;
    while i < args.len() {
        let k = args[i].trim_start_matches("--").to_string();
        let v = args.get(i + 1).cloned().unwrap_or_else(|| die(&format!("--{k} needs a value")));
        m.insert(k, v);
        i += 2;
    }
    m
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("info") => {
            let p = Params::parse(args.get(1).map(String::as_str).unwrap_or(""))
                .unwrap_or_else(|e| die(&e));
            let u = Universe::new(p, 256).unwrap_or_else(|e| die(&e));
            println!("spec      {}", lq16::params::SPEC_ID);
            println!("params    sha256:{}", hex(&u.h_params));
            println!("kernel    sha256:{}", hex(&u.h_kernel));
            println!("lut       sha256:{}", hex(&u.h_lut));
            println!("max_w     {}", u.kernel.weights.iter().max().unwrap());
            println!("nonzero   {}", u.kernel.taps().len());
            if let Some(dir) = args.get(2) {
                std::fs::create_dir_all(dir).unwrap_or_else(|e| die(&e.to_string()));
                std::fs::write(format!("{dir}/params.lq16"), lq16::records::params_record(&u.params)).unwrap();
                std::fs::write(format!("{dir}/kernel.lq16"), lq16::records::kernel_record(&u.kernel)).unwrap();
                std::fs::write(format!("{dir}/lut.lq16"), lq16::records::lut_record(&u.lut)).unwrap();
            }
        }
        Some("run") => {
            let f = flags(&args[1..]);
            let p = Params::parse(f.get("params").unwrap_or_else(|| die("--params required")))
                .unwrap_or_else(|e| die(&e));
            let bytes = std::fs::read(f.get("state").unwrap_or_else(|| die("--state required")))
                .unwrap_or_else(|e| die(&e.to_string()));
            let (n, state) = parse_state_record(&bytes).unwrap_or_else(|e| die(&e));
            let steps: u64 = f.get("steps").unwrap_or_else(|| die("--steps required")).parse()
                .unwrap_or_else(|_| die("bad --steps"));
            let threads: u32 = f.get("threads").map(|s| s.parse().unwrap()).unwrap_or(4);
            let mut u = Universe::new(p, n).unwrap_or_else(|e| die(&e));
            if f.get("algo").map(String::as_str) == Some("ntt") {
                u.use_ntt();
            }
            let res = u.run(state, steps, threads);
            println!("spec      {}", res.spec);
            println!("params    sha256:{}", hex(&u.h_params));
            println!("kernel    sha256:{}", hex(&u.h_kernel));
            println!("lut       sha256:{}", hex(&u.h_lut));
            println!("n         {n}");
            println!("steps     {steps}");
            println!("chain     sha256:{}", hex(&res.chain));
            let final_rec = state_record(n, &res.final_state);
            println!("final     sha256:{}", hex(&sha256(&final_rec)));
            let mass: u64 = res.final_state.iter().map(|&a| a as u64).sum();
            println!("mass      {mass}");
            if let Some(out) = f.get("out") {
                std::fs::write(out, &final_rec).unwrap_or_else(|e| die(&e.to_string()));
            }
            if let Some(pts) = f.get("points") {
                let txt: String =
                    res.points.iter().map(|(k, h)| format!("{k}\tsha256:{}\n", hex(h))).collect();
                std::fs::write(pts, txt).unwrap_or_else(|e| die(&e.to_string()));
            }
        }
        Some("soup") => {
            // Deterministic soup for public test vectors: SHA-256 counter-mode PRNG
            // (not normative; spec §9.2 note). Patch of size K centred in N×N,
            // cell = floor(r16 * amp / 65535), r16 from the PRNG stream.
            let f = flags(&args[1..]);
            let n: u32 = f.get("n").map(|s| s.parse().unwrap()).unwrap_or(256);
            let k: u32 = f.get("size").map(|s| s.parse().unwrap()).unwrap_or(40);
            let amp: u32 = f.get("amp").map(|s| s.parse().unwrap()).unwrap_or(65535);
            let seed: u64 = f.get("seed").map(|s| s.parse().unwrap()).unwrap_or(1);
            let out = f.get("out").unwrap_or_else(|| die("--out required"));
            lq16::params::validate_n(n, 5).unwrap_or_else(|e| die(&e));
            if k > n || amp > 65535 { die("bad --size or --amp"); }
            let mut state = vec![0u16; (n * n) as usize];
            let (mut ctr, mut pool, mut idx) = (0u64, [0u8; 32], 32usize);
            let x0 = (n - k) / 2;
            for y in 0..k {
                for x in 0..k {
                    if idx == 32 {
                        let mut m = b"LQ16SOUP".to_vec();
                        m.extend_from_slice(&seed.to_le_bytes());
                        m.extend_from_slice(&ctr.to_le_bytes());
                        pool = sha256(&m);
                        ctr += 1;
                        idx = 0;
                    }
                    let r16 = u16::from_le_bytes([pool[idx], pool[idx + 1]]) as u32;
                    idx += 2;
                    state[((x0 + y) * n + x0 + x) as usize] = (r16 * amp / 65535) as u16;
                }
            }
            let rec = state_record(n, &state);
            std::fs::write(out, &rec).unwrap_or_else(|e| die(&e.to_string()));
            println!("state     sha256:{}", hex(&sha256(&rec)));
        }
        _ => die("usage: lq16 info <params> | lq16 run --params .. --state .. --steps L"),
    }
}
