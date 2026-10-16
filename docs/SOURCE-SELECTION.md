# Source selection and evidence

Quill searches fetched page text. It does not certify the truth of the page, infer
accuracy from political alignment, or label an entire publisher as verified.
Reporting, commentary, satire, product documentation and social posts can all
contain relevant words without providing reliable evidence for a claim.

Source selection is explicit and local. `sources.txt` lists the URLs requested for
crawling. An optional `source-catalog.json` adds names, subject categories and links
to publisher information; these appear under Sources → Source selection notes.
Publisher descriptions and ownership disclosures are self-reported information,
not an independent audit. Catalog categories do not change ranking.

When evaluating sources, look for original documents or data, named authors,
traceable quotations, clear corrections, transparent funding/ownership and a
separation between reporting and opinion. These are review criteria, not a
currently implemented automatic truth score. Assess a particular claim against
its supporting and contradicting evidence; agreement between sites copying one
article is not independent corroboration.

For example, [Reason's description](https://reason.com/about/),
[The American Conservative's mission](https://www.theamericanconservative.com/about-us/),
[Ars Technica's newsroom information](https://arstechnica.com/about-us/), and
[BleepingComputer's ethics statement](https://www.bleepingcomputer.com/ethics-statement/)
make their stated approach inspectable. Linking these statements is not a guarantee
about their articles.

## What result metadata establishes

Newly crawled pages record when Quill fetched them. Where available, the parser
also retains a publisher-supplied publication date, explicitly labeled as such.
The fetch date is not the publication date. Older entries can have no date until
re-crawled. Neither field verifies the article's claims. Search results show the
canonical source URL, actual extracted text, matched terms and scoring details;
there is no generated answer presented as reporting.

The parser rejects some common bot-challenge titles instead of treating those
pages as articles. This is a limited heuristic, not comprehensive detection of
paywalls, misinformation, spam or anti-bot pages. Links inside main/article content
are queued before navigation links to improve the usefulness of small crawl budgets.

## Local exclusions

Sources → Excluded domains saves `excluded-domains.txt` beside the index. Rules
match a whole domain and its subdomains, never unrelated domains with similar
names. They apply before a crawl and when loading results/statistics. Defaults are
empty; no publishers are globally blacklisted in Quill's source code.

Exclusion hides existing documents from search but does not immediately erase their
stored text. A subsequent successful collection saves the filtered index. To
remove specific stored pages immediately, use `quill-search remove INDEX URL...`.
Exports and external backups are separate copies. Exclusions do not block opening
a website in Quill's embedded browser.

## Crawl coverage and transport

A saved source is not proof that it was indexed. Consult the crawl log for fetched,
added, excluded and skipped counts. Crawls stop scheduling further page requests
after a 60-second budget, saving completed pages; the log marks partial runs. Social platforms often require JavaScript or
login, disallow bots, or expose only thin public landing pages. Indexing a public
landing page does not index its videos, streams, comments or private posts.

The crawler validates TLS using Rustls and the operating system's trusted
certificate store. This supports networks whose certificate authorities are
already configured on the machine without disabling certificate checks. Managed
network filtering still applies. Quill does not install certificate authorities,
ignore certificate errors, bypass access controls, or send queries to a search API.
