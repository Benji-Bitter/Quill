<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/branding/white%20icon.png">
  <img src="assets/branding/black%20icon.png" alt="Quill" width="200">
</picture>

# Quill

Independent search, on your Mac.

Quill Search is a standalone native macOS app with its own Rust crawler, parser,
inverted index and ranking engine. It fetches public websites directly and searches
the saved index locally. No hosted backend, listening server, search API, ads or
query uploads. Coverage is limited to the pages you crawl; this is not a worldwide
web index.

## Open Quill Search on macOS

Install Xcode command-line tools, the pinned Rust toolchain and Python 3.11+:

```sh
python3 tools/build_search_app.py --open
```

The app is built at `dist/Quill Search.app`. Open **Sources**, enter website URLs,
and select **Crawl websites**. Search the downloaded pages from the main window.
Results include source links, snippets and inspectable ranking details. The app
supports local index export and deletion. No Firefox installation is needed.

Search uses the same BM25 rules for every publisher, with no paid or political
boosts. Source selection and ranking choices still introduce bias; Quill does not
claim to establish truth. See [standalone search](docs/SEARCH-APP.md).

The repository also retains the earlier experimental Firefox browser host,
privacy rules, encrypted vault core and signed-update verification library.
A hardened Quill browser distribution is not yet built.

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
See [building](docs/BUILDING.md) for bootstrap instructions and browser limits.

Search a crawl you control:

```sh
cargo run -p quill-search -- crawl .quill/index.json https://your-site.example/
cargo run -p quill-search -- query .quill/index.json 'your query'
```

Crawls stay on the seed origin, obey a conservative robots policy, and stop at a
fixed page limit. Search is entirely local after crawling. No commercial search
provider is used. See [search](docs/SEARCH.md).

On managed computers, an extension policy may block the Quill shell. The launcher
then clearly reports a homepage-only fallback: browsing still works, but Quill URL
cleaning and local new-tab search do not. It never overrides administrator policy.

## Status

| Area | State |
| --- | --- |
| Browsing | Experimental Firefox host: real tabs, HTTPS, navigation, history, bookmarks and existing Firefox tools |
| Quill chrome | Development CSS, supplied branding, minimal extension new tab |
| Privacy | Rust URL/rule core and development extension; no whole-engine network guarantee |
| Search | Native macOS UI, multi-site crawling, persisted custom index, lexical ranking and score explanations |
| Vault | Experimental authenticated encryption and local storage; no autofill or passkey store |
| Updates | Offline signature/hash verification; no downloader or installer |
| Distributed/semantic search, evidence | Design only |
| Independent Quill packages | Not built or signed |

[Architecture](docs/ARCHITECTURE.md) · [Privacy](docs/PRIVACY-PRINCIPLES.md) ·
[Threat model](docs/THREAT-MODEL.md) · [Measurements](docs/PERFORMANCE.md) · [Contributing](CONTRIBUTING.md)

Quill-owned source and supplied branding are distributed under [MPL-2.0](LICENSE).
Mozilla source retains its own notices; Mozilla trademarks are not licensed here.
