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

pub fn crawl(seed: &str, max_pages: usize) -> Result<(Index, Report)> {
    if max_pages == 0 || max_pages > 100 {
        return Err(invalid("crawl page limit must be 1..100"));
    }
    let seed = canonicalize(seed)?;
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .redirect(Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| invalid("could not create HTTP client"))?;
    let mut robots_url = seed.clone();
    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);
    let response = client
        .get(robots_url)
        .send()
        .map_err(|_| invalid("robots request failed; refusing crawl"))?;
    let robots = match response.status().as_u16() {
        200 => String::from_utf8(body(response, 512 * 1024)?)
            .map_err(|_| invalid("robots must be UTF-8"))?,
        404 | 410 => String::new(),
        _ => return Err(invalid("robots unavailable or redirects; refusing crawl")),
    };
    // Crawl-delay is nonstandard. Refuse rather than silently violate it.
    if robots
        .lines()
        .any(|line| line.trim().to_ascii_lowercase().starts_with("crawl-delay:"))
    {
        return Err(invalid(
            "crawl-delay requires a configured scheduler; refusing crawl",
        ));
    }
    let mut queue = VecDeque::from([seed.clone()]);
    let mut seen = BTreeSet::new();
    let mut index = Index::default();
    let mut report = Report::default();
    let mut last_request = Instant::now();
    while let Some(url) = queue.pop_front() {
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
        std::thread::sleep(Duration::from_millis(500).saturating_sub(last_request.elapsed()));
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
        let page = match body(response, MAX_HTML_BYTES)
            .and_then(|bytes| quill_search_parser::parse(&url, &bytes))
        {
            Ok(p) => p,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
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
