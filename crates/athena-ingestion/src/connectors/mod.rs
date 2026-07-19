//! One module per Version 1 integration (07_INTEGRATIONS.md §1). Every
//! connector is independently usable — none imports another — so that
//! "the application must continue functioning when every external
//! service is disconnected" holds structurally, not just by
//! convention: nothing here can fail to compile or panic at import time
//! just because a sibling connector is unconfigured.
//!
//! Each connector exposes plain functions that return typed data or an
//! `IngestionError` — no connector here touches SQL or Tauri directly
//! (those live in `athena-data` and `athena-app` respectively, per the
//! dependency graph in `athena-ingestion`'s own Cargo.toml). This keeps
//! every connector unit-testable without a database or a running app.

pub mod calendar_ics;
pub mod codeforces;
pub mod csv_import;
pub mod github;
pub mod leetcode;
pub mod pdf_import;

// 07_INTEGRATIONS.md §1.8-§1.10 (2026-07-17 OAuth amendment). `oauth2`
// is a shared, connector-agnostic utility (token exchange/refresh, PKCE)
// — not a connector itself — so gmail/google_classroom/notion each
// import it independently without importing one another, preserving
// this module's own "none imports another" rule at the connector level.
pub mod google_classroom;
pub mod gmail;
pub mod notion;
pub mod oauth2;
