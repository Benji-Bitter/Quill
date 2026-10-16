use quill_search_common::{
    Document, MAX_HTML_BYTES, MAX_TEXT_CHARS, Result, canonicalize, invalid,
};
use scraper::{Html, Selector};
use url::Url;

pub struct Page {
    pub document: Document,
    pub links: Vec<Url>,
    pub noindex: bool,
    pub nofollow: bool,
}

pub fn parse(url: &Url, bytes: &[u8]) -> Result<Page> {
    if bytes.len() > MAX_HTML_BYTES {
        return Err(invalid("HTML exceeds size limit"));
    }
    // Reject unsupported encodings rather than silently corrupting indexed text.
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("HTML must be UTF-8"))?;
    let html = Html::parse_document(text);
    let select = |s| Selector::parse(s).expect("static selector");
    let title = html
        .select(&select("title"))
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();
    if [
        "just a moment...",
        "access denied",
        "attention required! | cloudflare",
        "robot check",
        "verify you are human",
    ]
    .contains(&title.trim().to_ascii_lowercase().as_str())
    {
        return Err(invalid("access challenge is not an article"));
    }
    let published = html.select(&select("meta[property='article:published_time'], meta[name='date'], meta[itemprop='datePublished']"))
        .find_map(|e| e.value().attr("content"))
        .map(|s| s.chars().take(128).collect());
    let root = html
        .select(&select("main, article"))
        .next()
        .or_else(|| html.select(&select("body")).next());
    let mut body = String::new();
    if let Some(root) = root {
        for node in root.descendants() {
            if let Some(text) = node.value().as_text() {
                let hidden = node
                    .ancestors()
                    .filter_map(scraper::ElementRef::wrap)
                    .any(|e| {
                        matches!(
                            e.value().name(),
                            "script" | "style" | "nav" | "footer" | "noscript" | "template"
                        ) || e.value().attr("hidden").is_some()
                    });
                if !hidden {
                    body.push_str(text);
                    body.push(' ');
                }
            }
        }
    }
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut canonical = url.clone();
    if let Some(link) = html
        .select(&select("link[rel=canonical]"))
        .next()
        .and_then(|e| e.value().attr("href"))
        && let Ok(candidate) = url.join(link).and_then(|u| Url::parse(u.as_str()))
        && candidate.origin() == url.origin()
    {
        canonical = canonicalize(candidate.as_str())?;
    }
    let mut links = Vec::new();
    let article_links = select("main a[href], article a[href]");
    let all_links = select("a[href]");
    let mut seen = std::collections::BTreeSet::new();
    for element in html
        .select(&article_links)
        .chain(html.select(&all_links))
        .take(2000)
    {
        if element.value().attr("rel").is_some_and(|r| {
            r.split_whitespace()
                .any(|s| s.eq_ignore_ascii_case("nofollow"))
        }) {
            continue;
        }
        if let Some(link) = element
            .value()
            .attr("href")
            .and_then(|v| url.join(v).ok())
            .and_then(|u| canonicalize(u.as_str()).ok())
            && seen.insert(link.as_str().to_owned())
            && links.len() < 1000
        {
            links.push(link);
        }
    }
    let robots = html
        .select(&select("meta[name]"))
        .filter(|e| {
            e.value().attr("name").is_some_and(|n| {
                n.eq_ignore_ascii_case("robots") || n.eq_ignore_ascii_case("quillbot")
            })
        })
        .filter_map(|e| e.value().attr("content"))
        .collect::<Vec<_>>()
        .join(",")
        .to_ascii_lowercase();
    let directives: Vec<_> = robots
        .split(|c: char| c == ',' || c.is_whitespace())
        .collect();
    Ok(Page {
        document: Document {
            url: canonical.into(),
            title: title.chars().take(512).collect(),
            text: body.chars().take(MAX_TEXT_CHARS).collect(),
            fetched_at: None,
            published_at: published,
        },
        links,
        noindex: directives.contains(&"noindex") || directives.contains(&"none"),
        nofollow: directives.contains(&"nofollow") || directives.contains(&"none"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_content_not_scripts() {
        let page = parse(&Url::parse("https://example.org/a").unwrap(), br#"<title>A &amp; B</title><meta name="robots" content="noindex"><main>Hello <b>world</b><script>secret</script><nav>noise</nav><a href="/b#x">link</a></main>"#).unwrap();
        assert_eq!(page.document.title, "A & B");
        assert_eq!(page.document.text, "Hello world link");
        assert_eq!(page.links[0].as_str(), "https://example.org/b");
        assert!(page.noindex);
    }
    #[test]
    fn rejects_challenges_and_preserves_publisher_date() {
        let url = Url::parse("https://example.org/").unwrap();
        assert!(
            parse(
                &url,
                b"<title>Just a moment...</title><body>Checking your browser</body>"
            )
            .is_err()
        );
        let page = parse(&url, b"<meta property='article:published_time' content='2026-10-16T00:00:00Z'><nav><a href='/about'>About</a></nav><article>News<a href='/story'>Story</a></article>").unwrap();
        assert_eq!(
            page.document.published_at.as_deref(),
            Some("2026-10-16T00:00:00Z")
        );
        assert_eq!(page.links[0].path(), "/story");
    }
    #[test]
    fn ignores_external_canonical_and_rejects_big_input() {
        let url = Url::parse("https://example.org/").unwrap();
        assert_eq!(
            parse(
                &url,
                br#"<link rel="canonical" href="https://evil.example/"><body>text"#
            )
            .unwrap()
            .document
            .url,
            url.as_str()
        );
        assert!(parse(&url, &vec![b'x'; MAX_HTML_BYTES + 1]).is_err());
        assert!(parse(&url, &[255]).is_err());
    }
}
