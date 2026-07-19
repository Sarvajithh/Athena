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

    /// OAuth/access token is invalid, expired, or revoked.
    AuthExpired(String),

    /// Remote service rejected requests due to rate limiting.
    RateLimited(String),
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
            IngestionError::AuthExpired(msg) => {
                write!(f, "authentication expired: {}", msg)
            }
            IngestionError::RateLimited(msg) => {
                write!(f, "rate limited: {}", msg)
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