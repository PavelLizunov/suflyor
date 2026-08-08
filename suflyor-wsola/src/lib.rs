//! Minimal pitch-preserving WSOLA for suflyor speech playback.
//!
//! Derived from the `Wsola` implementation in `timestretch` 0.5.0 by Rob
//! Morgan, under the MIT license. See `NOTICE` and `LICENSE.upstream-MIT`.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod error;
mod wsola;

pub use error::WsolaError;
pub use wsola::Wsola;
