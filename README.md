<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/branding/white%20icon.png">
  <img src="assets/branding/black%20icon.png" alt="Quill" width="96">
</picture>

# Quill

Web browser. Nothing else.

Quill is an early, open-source browser project built toward a Firefox-derived
Gecko distribution. This repository contains Quill's code, not a vendored copy of
Firefox. It is not yet a production browser or an independent public search service.

The first milestone includes an experimental Firefox development profile and
extension, a bounded local crawler and custom inverted search index, URL privacy
rules, an encrypted vault core, and offline signed-update verification. These
components do not imply that a hardened Quill distribution exists.

## Run

Install Rust using the pinned toolchain and Python 3.11 or later.

```sh
cargo test --workspace --locked
cargo build --workspace --release --locked
python3 tools/search_demo.py
python3 tools/browser.py --firefox /path/to/firefox
```

On macOS, pass the executable inside Firefox.app:
`/Applications/Firefox.app/Contents/MacOS/firefox`.
The launcher creates a separate development profile, never your normal profile.
See [building](docs/BUILDING.md) for the engine download helper and browser limits.

Search a crawl you control:

```sh
cargo run -p quill-search -- crawl .quill/index.json https://your-site.example/
cargo run -p quill-search -- query .quill/index.json 'your query'
```

Crawls stay on the seed origin, obey a conservative robots policy, and stop at a
fixed page limit. Search is entirely local after crawling. No commercial search
provider is used. See [search](docs/SEARCH.md).

## Status

| Area | State |
| --- | --- |
| Browsing | Experimental Firefox host: real tabs, HTTPS, navigation, history, bookmarks and existing Firefox tools |
| Quill chrome | Development CSS, supplied branding, minimal extension new tab |
| Privacy | Rust URL/rule core and development extension; no whole-engine network guarantee |
| Search | Runnable controlled crawler, parser, persisted inverted index and lexical ranking |
| Vault | Experimental authenticated encryption and local storage; no autofill or passkey store |
| Updates | Offline signature/hash verification; no downloader or installer |
| Distributed/semantic search, evidence | Design only |
| Independent Quill packages | Not built or signed |

[Architecture](docs/ARCHITECTURE.md) · [Privacy](docs/PRIVACY-PRINCIPLES.md) ·
[Threat model](docs/THREAT-MODEL.md) · [Contributing](CONTRIBUTING.md)

Quill-owned source and supplied branding are distributed under [MPL-2.0](LICENSE).
Mozilla source retains its own notices; Mozilla trademarks are not licensed here.
