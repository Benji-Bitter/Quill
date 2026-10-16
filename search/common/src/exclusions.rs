use crate::{Result, canonicalize, invalid};
use std::{fs::File, io::Read, path::Path};

/// User-local policy beside the index, never a built-in publisher blacklist.
pub fn read(index: &Path) -> Result<Vec<String>> {
    let path = index.with_file_name("excluded-domains.txt");
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error.into()),
    };
    let mut text = String::new();
    file.take(65_537).read_to_string(&mut text)?;
    if text.len() > 65_536 {
        return Err(invalid("exclusion file too large"));
    }
    parse(&text)
}

pub fn parse(text: &str) -> Result<Vec<String>> {
    let mut domains = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
    {
        let domain = line.to_ascii_lowercase();
        let url = canonicalize(&format!("https://{domain}/"))?;
        if domain.split('.').any(|label| {
            label.is_empty()
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }) || domain.len() > 253
            || url.host_str() != Some(domain.as_str())
            || url.port().is_some()
        {
            return Err(invalid(
                "exclusions must be plain domain names, one per line",
            ));
        }
        domains.push(domain);
        if domains.len() > 200 {
            return Err(invalid("too many excluded domains"));
        }
    }
    domains.sort();
    domains.dedup();
    Ok(domains)
}

pub fn matches(url: &str, domains: &[String]) -> bool {
    canonicalize(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .is_some_and(|host| {
            let host = host.trim_end_matches('.');
            domains
                .iter()
                .any(|domain| host == domain || host.ends_with(&format!(".{domain}")))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_only_domain_boundaries() {
        let rules = parse("# local selection\nABC.net.au\nabcnews.go.com\n").unwrap();
        for url in [
            "https://abc.net.au/a",
            "https://www.abc.net.au/a",
            "https://abcnews.go.com./story",
        ] {
            assert!(matches(url, &rules));
        }
        for url in [
            "https://notabc.net.au/",
            "https://abc.net.au.evil.test/",
            "https://go.com/",
        ] {
            assert!(!matches(url, &rules));
        }
        for bad in [
            "https://abc.net.au",
            "abc.net.au/path",
            "a.test:444",
            "*.test",
        ] {
            assert!(parse(bad).is_err());
        }
    }
}
