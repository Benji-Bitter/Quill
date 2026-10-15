# Performance baseline

Measured on 15 October 2026 on an Apple M4, 16 GiB RAM, macOS 26.5.1.
Firefox 157.0.1 runs headless with a fresh development profile. The local managed
extension policy blocks Quill's extension, so these browser numbers describe the
explicit homepage fallback, not the full extension or a packaged Quill build.

| Measurement | Observed |
| --- | ---: |
| Process start through loaded homepage | 1.750 s |
| New tab plus homepage load, median of five | 126.7 ms |
| New-tab sample range | 62.1–136.0 ms |
| Process-tree RSS after 15 s settling | 1,840,480 KiB |
| CPU over the following 5.03 s | 0.25 CPU seconds; 4.97% of one core |
| Three-document index load, release CLI | 530 µs |
| In-memory lexical query p50, 1,000 samples | 4.46 µs |
| In-memory lexical query p95, 1,000 samples | 5.58 µs |

Browser timings include automation calls and filesystem loads. RSS sums processes
and double-counts shared pages; it is not unique physical memory. Exited child
processes are omitted from CPU sampling. Five seconds is a short idle window, and
upstream engine services remain active. These figures do not establish near-zero
idle CPU, low memory use, fast production startup or a web-scale search benchmark.
The three-document corpus is a correctness fixture, not a realistic search load.

Reproduce with:

```sh
python3 tools/search_demo.py
cargo run --release -p quill-search -- bench .quill/demo-index.json 'rust ownership'
python3 tools/browser_smoke.py --firefox /path/to/firefox
```

The browser test records JSON and a homepage screenshot under `.quill/measurements`.
Use repeated cold/warm runs, a representative corpus and longer idle periods before
making comparisons. Background network bytes require the separate network audit.
Tab sleeping/restoration is not implemented or measured; upstream low-memory tab
unloading alone does not establish the proposed Quill policy.
