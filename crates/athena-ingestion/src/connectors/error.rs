use std::fmt;

/// Typed error enum for `athena-ingestion`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestionError {
    /// Generic fallback.
    Unspecified,

    /// Parsing/formatting errors.
    Parse(String),

    /// Network or HTTP errors.
    Network(String),

    /// Missing API key, token, or other required configuration.
    NotConfigured(String),

    /// Remote service rejected requests due to rate limiting.
    RateLimited(String),

    /// An OAuth access token was rejected or has expired
    /// (07_INTEGRATIONS.md §1.8-§1.10). Distinct from `NotConfigured`
    /// (no token stored at all) and from `Network` (a token-independent
    /// failure) so the caller knows specifically that a refresh attempt
    /// — or, failing that, asking the user to reconnect — is the right
    /// next step, not a generic retry.
    AuthExpired(String),
}

impl fmt::Display for IngestionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IngestionError::Unspecified => {
                write!(f, "unspecified ingestion error")
            }
            IngestionError::Parse(msg) => {
                write!(f, "parse error: {}", msg)
            }
            IngestionError::Network(msg) => {
                write!(f, "network error: {}", msg)
            }
            IngestionError::NotConfigured(msg) => {
                write!(f, "not configured: {}", msg)
            }
            IngestionError::RateLimited(msg) => {
                write!(f, "rate limited: {}", msg)
            }
            IngestionError::AuthExpired(msg) => {
                write!(f, "auth expired: {}", msg)
            }
        }
    }
}

impl std::error::Error for IngestionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unspecified() {
        assert_eq!(
            IngestionError::Unspecified.to_string(),
            "unspecified ingestion error"
        );
    }

    #[test]
    fn display_parse() {
        assert_eq!(
            IngestionError::Parse("bad csv".into()).to_string(),
            "parse error: bad csv"
        );
    }

    #[test]
    fn display_network() {
        assert_eq!(
            IngestionError::Network("offline".into()).to_string(),
            "network error: offline"
        );
    }
}