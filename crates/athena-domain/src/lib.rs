//! `athena-domain` — pure reasoning rules, zero I/O.
//!
//! Per Master Spec §4.4/§4.5, this crate depends on nothing internal and
//! nothing beyond the Rust standard library (PROJECT_RULES.md Immutable
//! Rule #4). Sprint S01 (Foundation Scaffold) intentionally ships this
//! crate empty of domain logic — no `priority/`, `bottleneck/`, `drift/`,
//! `deep_work/`, or `divergence/` submodules yet. Those are added by the
//! first sprint that actually needs them (SPRINT1_SPEC.md §0/§1,
//! PROJECT_RULES.md Immutable Rule #7's "schema/module change is its own
//! reviewed deliverable" spirit applied to module scaffolding).

pub mod error;
pub mod planner;
pub mod priority;

pub use error::DomainError;