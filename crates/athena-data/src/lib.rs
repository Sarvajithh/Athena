//! `athena-data` — the only crate allowed to write SQL; owns migrations
//! (Master Spec §4.5, §5).
//!
//! S01 (Foundation Scaffold) ships: the SQLite connection/WAL bootstrap,
//! the migration runner wiring, the per-crate error skeleton, and an
//! empty repositories module reserved for the first real repository
//! (SPRINT1_SPEC.md §2). No domain tables exist yet — only the
//! migration-bookkeeping table refinery itself requires (PROJECT_RULES.md
//! Immutable Rule #7).

pub mod connection;
pub mod error;
pub mod repositories;

pub use error::DataError;

// Embeds the SQL migrations directory at compile time so the runner in
// `connection.rs` can apply them without relying on a runtime file path
// (important for a bundled desktop app where the working directory at
// launch is not guaranteed to be the repo root).
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

pub(crate) use embedded::migrations;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        // Trivial compile/sanity test per SPRINT1_SPEC.md §7
        // ("each of the six crates has at least one trivial test").
        assert!(true);
    }
}
