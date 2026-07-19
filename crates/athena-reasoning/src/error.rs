use std::fmt;

/// Typed error enum for `athena-reasoning`.
///
/// Per Implementation Plan §6, every crate owns its own typed error enum.
/// These variants cover the two fallible seams the AI layer actually has
/// (06_AI_ENGINE.md §9/§10): a provider that can't be reached at all,
/// and a provider that responded but produced something Stage 5 can't
/// verify. Neither variant is fatal to the caller — `pipeline::Synthesizer`
/// catches both and falls through to the next provider, and ultimately to
/// the zero-LLM template (§10), so a `ReasoningError` reaching
/// `athena-app` should be rare and is always non-blocking when it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningError {
    /// The provider (cloud or local) could not be reached, timed out, or
    /// returned a transport-level failure — never raised for "the model
    /// declined," since Stage 4 never asks the model to decide anything.
    ProviderUnavailable(String),
    /// The provider responded, but the response wasn't the JSON shape
    /// `output_schema` requires (06_AI_ENGINE.md §7.4) — malformed JSON,
    /// a missing required field, or a citation that isn't even a number.
    /// Distinct from a grounding failure: this is "unparseable," not
    /// "parseable but unverifiable."
    ResponseSchemaInvalid(String),
    /// Stage 5's grounding check (06_AI_ENGINE.md §3): the response cited
    /// an evidence ID absent from the Stage 1 payload, or contained a
    /// claim the payload doesn't support. Raised once per attempt; the
    /// pipeline retries a single time with a stricter prompt before
    /// giving up on this provider for this call.
    GroundingCheckFailed(String),
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasoningError::ProviderUnavailable(msg) => write!(f, "LLM provider unavailable: {msg}"),
            ReasoningError::ResponseSchemaInvalid(msg) => write!(f, "LLM response failed schema validation: {msg}"),
            ReasoningError::GroundingCheckFailed(msg) => write!(f, "LLM response failed grounding check: {msg}"),
        }
    }
}

impl std::error::Error for ReasoningError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_error_variants_display_a_useful_message() {
        assert!(ReasoningError::ProviderUnavailable("timeout".into())
            .to_string()
            .contains("unavailable"));
        assert!(ReasoningError::ResponseSchemaInvalid("not json".into())
            .to_string()
            .contains("schema"));
        assert!(ReasoningError::GroundingCheckFailed("id 99 not in payload".into())
            .to_string()
            .contains("grounding"));
    }
}
