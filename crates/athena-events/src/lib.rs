//! `athena-events` — the Command/Event bus and interceptor registry,
//! including the Decision Challenge Layer (Master Spec §4.5, §4.6).
//!
//! S01 (Foundation Scaffold) ships this crate empty of dispatcher/
//! interceptor logic — that is a later sprint's deliverable
//! (SPRINT1_SPEC.md §2, "No interceptor chain, no dispatcher logic").

pub mod error;

pub use error::EventsError;
