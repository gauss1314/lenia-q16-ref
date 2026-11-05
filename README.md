# lenia-q16-ref

Reference implementation of the **lenia-q16** universe: classic single-channel
Lenia with 16-bit integer state, defined so that every implementation produces
bit-identical results on every platform. Spec draft: `lenia-q16/0.2-draft` (kernel weight total 2^20, D-028)
(kept in the design repository, `docs/spec/lenia-q16-draft.md`).

Status: Phase 0 prototype. Not for registration use (spec §1).

## What is here

- `src/` — integer-only implementation of spec §2–§8: `rne`, `isqrt`,
  fixed-point `expneg`, kernel generation with largest-remainder
  quantisation by d² class (weight total 2^20), the growth lookup table,
  direct-sum and NTT (two primes + CRT) updates, record serialisation,
  SHA-256 and the trajectory hash chain.
- `src/bin/lq16.rs` — CLI (`lq16 info`, `lq16 run`).
- `tools/lenia_f64.py` — non-normative float64 control used for comparisons
  and for culturing initial states.
- `wasm/` — C-ABI wrapper; `cargo build --release -p lq16-wasm --target wasm32-unknown-unknown`.
- `vectors/v0/vectors.tsv` — public determinism vectors (self-chosen parameters);
  `tools/check_vectors.sh [--wasm]` re-runs them natively and through WASM.
- `bench/v0/` — self-found benchmark structures (200 TU, routine CI);
  `vectors/g1/` — the full matrix (benchmarks and two random soups, 2000 TU each),
  `tools/check_g1.sh`, manual workflow `g1-full`.
- `web/vectors/standalone.html` — one-file self-test: open it from disk in desktop
  Chrome to run the vectors in that browser (built by `tools/make_standalone.py`).
- `web/explorer/` — WebGPU float32 preview for play-testing.
- `.github/workflows/ci.yml` — the determinism configurations: macOS ARM64 and
  Windows x86-64 native, WASM in Chromium (with the WASM float-instruction scan).
  `explorer.yml` checks the explorer's JavaScript spec kernel against Rust.

## Build and test

```
cargo test --release
cargo build --release
./target/release/lq16 info "R=13 T=10 mu=2600 sigma=360 beta=1/2,1/1"
```

`<params>`: `R`, `T`, `mu` and `sigma` in units of 1e-4, `beta` as `p/q` terms
from the inner ring outwards.

No external crates: SHA-256 is implemented in `src/sha256.rs` and checked
against the FIPS 180-4 vectors.

## Rules for contributors

- No floating point on the exact path, including generation (spec §7.3).
- No `usize`-width values in arithmetic; fixed-width integers only.
- Test vectors that involve species from the literature never go into this
  repository; they live in a separate private test repository.

## Licence

Dual-licensed under MIT or Apache-2.0, at your option.
