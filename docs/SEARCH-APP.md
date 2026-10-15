# Quill Search for macOS

Quill Search runs without a server. A native SwiftUI window calls Quill's Rust
executable through local pipes. The app has no Firefox dependency,
listening socket or hosted search provider. The Rust engine performs direct HTTPS
fetching only when a crawl is requested. Queries use only the on-device index.
Clicking a result opens its URL inside Quill using macOS system WebKit. The embedded
browser supports an editable address bar, back/forward, reload/stop and return to
search. Links requesting a new window open in the same view. Search results remain
in memory when returning from a website.

Websites execute JavaScript and make their own network requests inside WebKit;
this traffic is separate from the offline search engine. WebKit's normal TLS
validation remains enabled. Cookies and website storage use a nonpersistent session,
not Safari's profile; visiting pages does not add them to the search index. Returning
to search unloads the current page to stop hidden scripts and media. Camera and
microphone are denied. Downloads, extension support, multiple tabs and persistent
site logins are not implemented in this embedded browser. This is a macOS WebKit
integration, separate from the repository's experimental Gecko host.

Build with Rust, Python 3.11+ and Xcode command-line tools:

```sh
python3 tools/build_search_app.py --open
```

This builds an ad-hoc signed app for the host Mac at `dist/Quill Search.app`.
It is not a notarized release. The native UI currently targets macOS 14 or later;
the underlying Rust engine also builds on Windows and Linux.

## Using the index

Open Sources and enter complete HTTP(S) URLs, one per line. Choose the page budget
and start the crawl. The budget counts visited URLs, so redirects, exclusions and
non-HTML pages can leave fewer indexed pages. Each seed stays on its origin;
external links are not followed. Up to 20 seeds can be entered at once. Website
operators see the requesting IP address and QuillBot user agent.

Search works after the first successful site completes and continues to work
offline. A crawl can be stopped; completed sites remain saved, while the current
site's incomplete crawl is discarded. Re-crawling refreshes matching page URLs.
Pages no longer discovered remain in the index; automatic expiry is not implemented.
The interface reports failures instead of substituting results from another engine.
Example sources are merely editable suggestions, not an endorsement or a ranking
allowlist. Some sites block bots or change their robots policies.

Data lives in `~/Library/Application Support/Quill Search/web-index.json`, in a
user-only directory. Export copies the public-page index to a chosen file. Delete
removes the active index; it does not erase exported copies, backups or filesystem
history. No browsing history, account credentials or search-query history is stored
by this app. Do not crawl private token-bearing URLs you would not want in an export.

## Ranking and coverage

Ranking uses the custom inverted index and BM25: title term frequency has weight
three, body matches have weight one, and unusually repetitive documents receive a
penalty. Equal scores are ordered by URL. Each result exposes matched terms, total
score and repetition factor. No publisher reputation, political classification,
payment or user profile changes these scores. They measure lexical relevance,
not accuracy or factual support.

There is no unbiased view of the entire web here. Source selection, language,
coverage, extraction and ranking all affect results. The current index is limited
to 1,000 pages / 64 MiB, with up to 100 visited URLs per seed and ten returned hits.
It supports UTF-8 HTML text, not JavaScript-rendered pages, PDFs, stemming,
semantic retrieval or peer-to-peer discovery. A large independent web index needs
substantial crawl coverage and storage beyond this local implementation.

## Checks

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 tools/search_demo.py
python3 tools/build_search_app.py
python3 tools/test_search_app.py
python3 tools/test_browser_view.py
```

The native smoke test calls the actual Rust engine, checks real ranked results and
empty results, then deletes only a temporary index. Rust tests cover refresh/merge,
score explanations, exclusive writers and bounded crawl delays. CI builds the
macOS app and runs the model test alongside the existing cross-platform suite.
