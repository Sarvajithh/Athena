use std::fmt;

/// Typed error enum for `athena-events`.
///
/// Per Implementation Plan §6, every crate owns its own typed error enum.
/// S01 (SPRINT1_SPEC.md §1 Objective 6) ships no variants beyond what
/// compiles — the interceptor chain's fail-open/fail-closed error
/// semantics (Implementation Plan §6) are added once the dispatcher
/// itself exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventsError {
    /// Placeholder variant proving the enum compiles and is constructible.
    Unspecified,
}

impl fmt::Display for EventsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventsError::Unspecified => write!(f, "unspecified events error"),
        }
    }
}

impl std::error::Error for EventsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_error_is_constructible() {
        let err = EventsError::Unspecified;
        assert_eq!(err, EventsError::Unspecified);
        assert_eq!(err.to_string(), "unspecified events error");
    }
}
