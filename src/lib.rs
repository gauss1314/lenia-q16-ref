//! Reference implementation of the lenia-q16 universe, draft `lenia-q16/0.1-draft`; branch exp/kernel-2p20 runs the experimental `lenia-q16/0.2-k20` (kernel total 2^20).
//!
//! Everything on the exact path is integer arithmetic (spec §7.3). The spec text
//! lives in the design repository (docs/spec/lenia-q16-draft.md); section numbers
//! in comments refer to it.

pub mod expneg;
pub mod intmath;
pub mod kernel;
pub mod lut;
pub mod ntt;
pub mod params;
pub mod records;
pub mod sha256;
pub mod world;
