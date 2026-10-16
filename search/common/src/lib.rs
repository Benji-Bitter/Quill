pub mod exclusions;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

pub const MAX_HTML_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_INDEX_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_DOCUMENTS: usize = 1000;
pub const MAX_TEXT_CHARS: usize = 100_000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn invalid(message: &str) -> Error {
    Error::Invalid(message.to_owned())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

pub fn canonicalize(input: &str) -> Result<Url> {
    let mut url = Url::parse(input).map_err(|_| invalid("invalid URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid(
            "only HTTP(S) URLs without credentials are accepted",
        ));
    }
    url.set_fragment(None);
    // Query ordering, case, and tracking parameters can change the resource.
    Ok(url)
}

pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(64).flat_map(char::to_lowercase).collect())
        .collect()
}
pub fn content_hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_urls() {
        assert_eq!(
            canonicalize("HTTPS://EXAMPLE.org:443/a/../b?q=2&q=1#x")
                .unwrap()
                .as_str(),
            "https://example.org/b?q=2&q=1"
        );
        for bad in [
            "file:///etc/passwd",
            "https://u:p@example.org",
            "javascript:alert(1)",
            "//example.org",
        ] {
            assert!(canonicalize(bad).is_err());
        }
    }
    #[test]
    fn unicode_tokens() {
        assert_eq!(tokens("Hello—CAFÉ 東京"), ["hello", "café", "東京"]);
    }
}
