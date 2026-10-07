# Search

The current search engine crawls a controlled origin and queries a local custom
inverted index. It never proxies a commercial engine. Run:

```sh
python3 tools/search_demo.py
cargo run -p quill-search -- query .quill/demo-index.json 'rust ownership'
cargo run --release -p quill-search -- bench .quill/demo-index.json 'rust ownership'
```

The demo serves committed HTML on loopback, crawls it, checks robots exclusion,
canonical links, duplicates, noindex/nofollow and oversized responses, then starts
a separate query process. Results contain title, canonical URL, snippet and score.
They are computed from fetched content, not a stored answer fixture.

## Implemented pipeline

`common` owns URL validation, Unicode alphanumeric tokenization and the document
model. HTTP(S) only, no credentials, fragments removed, path normalization via
`url`; queries retain their order and meaning. Tracking removal is a separate
navigation policy and is deliberately not applied to crawler identity.

`crawler` checks robots.txt with Google's Rust robots matcher, then fetches at
most 30 URLs through the CLI (library maximum 100). It requests at most twice a
second, checks TLS certificates, disables implicit proxies and redirects, and
follows discovered redirects only on the seed origin after another robots check.
Robots 404/410 allow crawling; other non-200 responses fail closed. Crawl-delay
currently causes refusal because a configurable scheduler is not implemented.
Robots are bounded to 512 KiB; HTML to 2 MiB; each request has a 15-second timeout.
Compressed content is rejected to avoid decompression bombs. UTF-8 HTML only.

`parser` uses an HTML5 parser, extracts title and main/article/body text, excludes
scripts/navigation and hidden elements, and collects bounded links. Same-origin
canonical hints are accepted; off-origin hints are ignored. Meta robots and
X-Robots-Tag exclusions are respected conservatively. No JavaScript executes.
CSS visibility, sophisticated extraction and language identification are not built.

`index` stores document IDs, body positions and title term frequencies in a custom
inverted structure. Normalized exact content duplicates and duplicate canonical
URLs are excluded. Versioned JSON segments contain documents and postings; load
rebuilds postings and rejects inconsistent data. Atomic replacement avoids exposing
a partial write. Inputs are bounded to 64 MiB, 1,000 documents and 100,000 text
characters per document. This is a single-segment MVP, not a global-scale format.
Mmap, compression, multiple segments and background compaction are future work.

`ranking` uses BM25 with a title frequency weight of three. `spam` applies a modest
penalty to extreme token repetition. No political/source reputation table, paid
ranking, fabricated corroboration or model call is involved. These heuristics do
not establish truth, detect all SEO spam or identify original authorship.

`query` is the CLI. Its benchmark separates loading from 1,000 in-memory queries.
The new-tab extension separately searches Firefox history/bookmarks through local
WebExtension APIs. It does not import them into the Rust crawl index. Public and
private corpora must remain separate. A SQLite FTS5 history adapter, explicit saved
content ingestion and user-controlled export/delete are planned.

## Next ranking work

Add primary-source lineage, near-duplicate analysis, citation provenance,
information density and query-dependent freshness using inspectable features.
Measure with relevance judgments and adversarial corpora before adding weights.
Semantic matching belongs in `semantic/`, opt-in and offline; expensive reranking
should run only after a bounded lexical/semantic candidate stage. There is no
local preference profile or LLM currently running.

## Distributed search and evidence

There is no peer network. A later design may use libp2p, signed content-addressed
shards and replication, with local verification before indexing. Signatures
prove provenance, not factual accuracy. Raw queries must not be sent to peers as
a default; prefetching broad shards trades bandwidth for privacy. Peer discovery,
Sybil resistance, poisoned indexes, censorship, IP correlation, resource budgets
and query anonymity require a separate protocol review before implementation.

Evidence should preserve supporting and contradicting sources, primary-source
lineage, uncertainty and provenance locally. Planned statuses include verified,
strongly supported, supported, uncertain, disputed, misleading, contradicted,
fabricated, unverifiable and opinion. A status is a traceable assessment, never a
replacement for the evidence. No classifier is implemented: don't trust us; check us.
