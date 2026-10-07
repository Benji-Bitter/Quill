use serde::{Deserialize, Serialize};
use url::Url;

pub const TRACKING_PARAMETERS: &str = include_str!("../tracking-parameters.json");

/// Only clean explicit navigation URLs. Signed URLs and application APIs may break.
pub fn clean_navigation(input: &str) -> Result<String, url::ParseError> {
    let mut url = Url::parse(input)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Ok(input.to_owned());
    }
    let names: Vec<String> = serde_json::from_str(TRACKING_PARAMETERS).expect("bundled rules");
    // Preserve byte representation of all other parameters (including signatures).
    if let Some(query) = url.query() {
        let kept: Vec<&str> = query
            .split('&')
            .filter(|part| {
                let decoded = url::form_urlencoded::parse(part.as_bytes()).next();
                !decoded
                    .is_some_and(|(name, _)| names.iter().any(|n| n.eq_ignore_ascii_case(&name)))
            })
            .collect();
        let cleaned = kept.join("&");
        url.set_query(if cleaned.is_empty() {
            None
        } else {
            Some(&cleaned)
        });
    }
    Ok(url.into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Tracker,
    Ad,
    Analytics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub domain: String,
    pub category: Category,
}

pub struct Rules(Vec<Rule>);
impl Rules {
    pub fn from_json(input: &str) -> Result<Self, &'static str> {
        if input.len() > 1024 * 1024 {
            return Err("rules exceed limit");
        }
        let mut rules: Vec<Rule> = serde_json::from_str(input).map_err(|_| "invalid rules")?;
        if rules.len() > 10_000 {
            return Err("too many rules");
        }
        for rule in &mut rules {
            rule.domain.make_ascii_lowercase();
            if rule.domain.is_empty()
                || rule.domain.len() > 253
                || rule.domain.split('.').any(|label| {
                    label.is_empty()
                        || label.len() > 63
                        || label.starts_with('-')
                        || label.ends_with('-')
                        || !label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                })
            {
                return Err("invalid rule domain");
            }
        }
        Ok(Self(rules))
    }
    pub fn classify(&self, url: &Url) -> Option<&Category> {
        let host = url.host_str()?;
        self.0
            .iter()
            .find(|r| {
                host == r.domain
                    || host
                        .strip_suffix(&r.domain)
                        .is_some_and(|prefix| prefix.ends_with('.'))
            })
            .map(|r| &r.category)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Permission {
    Ask,
    Deny,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Permissions {
    pub camera: Permission,
    pub microphone: Permission,
    pub location: Permission,
    pub notifications: Permission,
    pub clipboard: Permission,
    pub sensors: Permission,
    pub bluetooth: Permission,
    pub usb: Permission,
    pub midi: Permission,
}
impl Default for Permissions {
    fn default() -> Self {
        Self {
            camera: Permission::Ask,
            microphone: Permission::Ask,
            location: Permission::Ask,
            notifications: Permission::Deny,
            clipboard: Permission::Ask,
            sensors: Permission::Deny,
            bluetooth: Permission::Deny,
            usb: Permission::Deny,
            midi: Permission::Deny,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_query_encoding_and_fragment() {
        assert_eq!(
            clean_navigation("https://example.org/?a=%2f+%20&utm_source=x&b=2&a=3#part").unwrap(),
            "https://example.org/?a=%2f+%20&b=2&a=3#part"
        );
        assert_eq!(
            clean_navigation("https://example.org/?%75tm_source=x").unwrap(),
            "https://example.org/"
        );
        assert_eq!(clean_navigation("about:blank").unwrap(), "about:blank");
        assert!(clean_navigation("not a URL").is_err());
    }
    #[test]
    fn idempotent_for_hostile_values() {
        for value in ["%", "%FF", "a&gclid=x", "💡", "a=b", "", "%26utm_source=x"] {
            let once =
                clean_navigation(&format!("https://example.org/?q={value}&fbclid=a")).unwrap();
            assert_eq!(clean_navigation(&once).unwrap(), once);
        }
    }
    #[test]
    fn domain_boundaries() {
        let rules = Rules::from_json(r#"[{"domain":"ads.example","category":"ad"}]"#).unwrap();
        assert_eq!(
            rules.classify(&Url::parse("https://a.ads.example/x").unwrap()),
            Some(&Category::Ad)
        );
        assert_eq!(
            rules.classify(&Url::parse("https://notads.example/").unwrap()),
            None
        );
        assert!(Rules::from_json(r#"[{"domain":"","category":"ad"}]"#).is_err());
    }
}
