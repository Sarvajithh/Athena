//! `athena-ingestion` — external connectors: Codeforces, ICS import, CSV
//! import (Master Spec §4.5). Isolated because ingestion is the most
//! likely thing to break over a 5-year horizon.
//!
//! S01 (Foundation Scaffold) ships this crate empty — no connector exists
//! yet (SPRINT1_SPEC.md §1, explicitly out of scope for this sprint).

pub mod error;
pub mod connectors;
pub use error::IngestionError;
