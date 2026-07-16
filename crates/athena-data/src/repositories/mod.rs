//! Repository interfaces live here, one per aggregate (Master Spec §4.5:
//! "one repository per aggregate; the only crate allowed to write SQL").
//!
//! S01 (Foundation Scaffold) reserved this location with no repository —
//! there was no domain table yet. The onboarding feature's V2 migration
//! adds the first domain tables, so this module now adds their
//! repositories: one per aggregate, matching 04_DATA_MODEL.md's table
//! list exactly.

pub mod course;
pub mod deadline;
pub mod decision;
pub mod disruption;
pub mod event_log;
pub mod profile;
pub mod semester;