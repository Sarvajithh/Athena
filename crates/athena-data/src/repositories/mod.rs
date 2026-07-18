//! Repository interfaces live here, one per aggregate (Master Spec §4.5:
//! "one repository per aggregate; the only crate allowed to write SQL").
//!
//! S01 (Foundation Scaffold) reserved this location with no repository —
//! there was no domain table yet. The onboarding feature's V2 migration
//! added the first domain tables (one repository per table, matching
//! 04_DATA_MODEL.md's list exactly); `disruption` followed with V3.
//! `integrations` is V4's addition (07_INTEGRATIONS.md) — one file
//! covering `data_sources` plus every connector's snapshot table, since
//! those seven tables are one aggregate in practice (sync status + the
//! typed data it produces), not seven independent domain concepts.

pub mod course;
pub mod deadline;
pub mod decision;
pub mod disruption;
pub mod event_log;
pub mod integrations;
pub mod profile;
pub mod routine;
pub mod semester;