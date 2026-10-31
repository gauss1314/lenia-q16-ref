# lenia-q16-ref

Reference implementation of the **lenia-q16** universe: classic single-channel
Lenia with 16-bit integer state, defined so that every implementation produces
bit-identical results on every platform. Spec draft: `lenia-q16/0.1-draft`
(kept in the design repository, `docs/spec/lenia-q16-draft.md`).

Status: Phase 0 prototype. Not for registration use (spec §1).

## What is here

- `src/` — integer-only implementation of spec §2–§8: `rne`, `isqrt`,
  fixed-point `expneg`, kernel generation with largest-remainder
  quantisation by d² class, the growth lookup table, direct-sum update,
  record serialisation, SHA-256 and the trajectory hash chain.
- `src/bin/lq16.rs` — CLI (`lq16 info`, `lq16 run`).
- `tools/lenia_f64.py` — non-normative float64 control used for comparisons
  and for culturing initial states.

## Build and test

```
cargo test --release
cargo build --release
./target/release/lq16 info "R=14 T=12 mu=1600 sigma=170 beta=1/1"
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
