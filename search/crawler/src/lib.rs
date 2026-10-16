use quill_search_common::{MAX_HTML_BYTES, Result, canonicalize, invalid};
use quill_search_index::Index;
use reqwest::{
    blocking::{Client, Response},
    header,
    redirect::Policy,
};
use std::{
    collections::{BTreeSet, VecDeque},
    io::Read,
    time::{Duration, Instant},
};
use url::Url;

pub const USER_AGENT: &str = "QuillBot/0.1 (+https://github.com/Benji-Bitter/Quill)";

#[derive(Debug, Default)]
pub struct Report {
    pub fetched: usize,
    pub indexed: usize,
    pub denied: usize,
    pub skipped: usize,
    pub time_limited: bool,
}

fn body(response: Response, limit: usize) -> Result<Vec<u8>> {
    if response
        .headers()
        .get(header::CONTENT_ENCODING)
        .is_some_and(|v| v != "identity")
    {
        return Err(invalid("compressed responses are not accepted"));
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(invalid("response exceeds size limit"));
    }
    let mut bytes = Vec::new();
    response.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("response exceeds size limit"));
    }
    Ok(bytes)
}

fn fetch_robots(client: &Client, seed: &Url) -> Result<String> {
    let mut url = seed
        .join("/robots.txt")
        .map_err(|_| invalid("invalid robots URL"))?;
    for attempt in 0..=5 {
        let response = client.get(url.clone()).send().map_err(|error| {
            invalid(&format!(
                "robots request failed; refusing crawl ({error:?})"
            ))
        })?;
        if response.status().is_redirection() {
            let next = response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| url.join(v).ok())
                .and_then(|u| canonicalize(u.as_str()).ok())
                .filter(|u| u.origin() == seed.origin())
                .ok_or_else(|| invalid("robots redirect leaves the seed origin; refusing crawl"))?;
            if attempt == 5 {
                return Err(invalid("too many robots redirects; refusing crawl"));
            }
            url = next;
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        return match response.status().as_u16() {
            200 => {
                let text = String::from_utf8(body(response, 512 * 1024)?)
                    .map_err(|_| invalid("robots must be UTF-8"))?;
                let prefix: String = text
                    .chars()
                    .take(512)
                    .flat_map(char::to_lowercase)
                    .collect();
                if prefix.contains("<html") || prefix.contains("<!doctype html") {
                    return Err(invalid("robots returned an HTML page; refusing crawl"));
                }
                Ok(text)
            }
            404 | 410 => Ok(String::new()),
            status => Err(invalid(&format!(
                "robots returned HTTP {status}; refusing crawl"
            ))),
        };
    }
    Err(invalid("robots redirect limit reached"))
}

pub fn crawl(seed: &str, max_pages: usize) -> Result<(Index, Report)> {
    if max_pages == 0 || max_pages > 100 {
        return Err(invalid("crawl page limit must be 1..100"));
    }
    let started = Instant::now();
    let budget = Duration::from_secs(60);
    let seed = canonicalize(seed)?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .redirect(Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| invalid("could not create HTTP client"))?;
    let robots = fetch_robots(&client, &seed)?;
    let delay = crawl_delay(&robots)?;
    let mut queue = VecDeque::from([seed.clone()]);
    let mut seen = BTreeSet::new();
    let mut index = Index::default();
    let mut report = Report::default();
    let mut last_request = Instant::now();
    while let Some(url) = queue.pop_front() {
        if started.elapsed() >= budget {
            report.time_limited = true;
            break;
        }
        if seen.len() >= max_pages {
            break;
        }
        if url.origin() != seed.origin() || !seen.insert(url.to_string()) {
            continue;
        }
        let mut matcher = robotstxt::DefaultMatcher::default();
        if !matcher.one_agent_allowed_by_robots(&robots, "QuillBot", url.as_str()) {
            report.denied += 1;
            continue;
        }
        let wait = delay.saturating_sub(last_request.elapsed());
        if started.elapsed().saturating_add(wait) >= budget {
            report.time_limited = true;
            break;
        }
        std::thread::sleep(wait);
        last_request = Instant::now();
        let response = match client
            .get(url.clone())
            .header(header::ACCEPT, "text/html")
            .send()
        {
            Ok(r) => r,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        report.fetched += 1;
        if response.status().is_redirection() {
            if let Some(next) = response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| url.join(v).ok())
                .and_then(|u| canonicalize(u.as_str()).ok())
                && next.origin() == seed.origin()
            {
                queue.push_back(next);
            }
            continue;
        }
        if response.status() != 200
            || !response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| {
                    v.split(';')
                        .next()
                        .is_some_and(|t| t.trim().eq_ignore_ascii_case("text/html"))
                })
        {
            report.skipped += 1;
            continue;
        }
        let header_robots = response
            .headers()
            .get("x-robots-tag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let mut page = match body(response, MAX_HTML_BYTES)
            .and_then(|bytes| quill_search_parser::parse(&url, &bytes))
        {
            Ok(p) => p,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        page.document.fetched_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| invalid("system clock is before Unix epoch"))?
                .as_secs(),
        );
        if !page.noindex
            && !header_robots.contains("noindex")
            && !header_robots.contains("none")
            && index.insert(page.document)?
        {
            report.indexed += 1;
        }
        if !page.nofollow && !header_robots.contains("nofollow") && !header_robots.contains("none")
        {
            for link in page.links {
                if queue.len() < 1000
                    && link.origin() == seed.origin()
                    && !seen.contains(link.as_str())
                {
                    queue.push_back(link);
                }
            }
        }
    }
    Ok((index, report))
}

/// A seed may deliberately address loopback for controlled tests. This crawler
/// must not be exposed as a server accepting URLs from untrusted clients.
pub fn seed_origin(seed: &str) -> Result<String> {
    let url: Url = canonicalize(seed)?;
    Ok(url.origin().ascii_serialization())
}

// Apply the largest declared delay conservatively, even for other user agents.
fn crawl_delay(robots: &str) -> Result<Duration> {
    let mut seconds = 0.5_f64;
    for line in robots.lines() {
        let line = line.split('#').next().unwrap_or("");
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("crawl-delay")
        {
            let requested: f64 = value
                .trim()
                .parse()
                .map_err(|_| invalid("invalid crawl-delay; refusing crawl"))?;
            if !requested.is_finite() || !(0.0..=30.0).contains(&requested) {
                return Err(invalid(
                    "crawl-delay exceeds supported 30 seconds; refusing crawl",
                ));
            }
            seconds = seconds.max(requested);
        }
    }
    Ok(Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn respects_delays_and_rejects_unbounded_values() {
        assert_eq!(crawl_delay("").unwrap(), Duration::from_millis(500));
        assert_eq!(
            crawl_delay("Crawl-delay: 2 # seconds\nCrawl-delay: 1").unwrap(),
            Duration::from_secs(2)
        );
        for value in ["NaN", "inf", "-1", "31", "no"] {
            assert!(crawl_delay(&format!("Crawl-delay: {value}")).is_err());
        }
    }
}
