use std::fmt;

/// Typed error enum for `athena-domain`.
///
/// Per Implementation Plan §6 ("Errors are typed, per-crate, and never
/// stringly-typed"), every crate owns its own error enum. S01
/// (SPRINT1_SPEC.md §1 Objective 6) ships this with no variants beyond
/// what is required to compile and be exercised by a trivial test —
/// later sprints add real variants only as they add real fallible
/// domain operations (e.g. `priority/`, `bottleneck/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// Placeholder variant proving the enum compiles and is constructible.
    /// Removed or superseded once the first real fallible domain
    /// operation is added in a later sprint.
    Unspecified,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::Unspecified => write!(f, "unspecified domain error"),
        }
    }
}

impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_error_is_constructible() {
        let err = DomainError::Unspecified;
        assert_eq!(err, DomainError::Unspecified);
        assert_eq!(err.to_string(), "unspecified domain error");
    }
}
