# Contributing

Use the pinned Rust toolchain and Python 3.11+. Run `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked`, and `python3 -m unittest discover -s tests`.
Run extension tests with `node --test tests/extension.test.mjs` (Node.js 24).
Keep changes focused and explain security boundaries. Never commit browsing
profiles, indexes, credentials, private signing keys, or captured user traffic.

Engine changes belong in `browser/`, independently of reusable Rust crates.
A Gecko patch needs an upstream revision and an integration test. Browser privacy
claims require packet-level evidence and engine tests, not just preference files.

Dependencies should solve a difficult problem, have a compatible license, and
have their lockfile reviewed. Cryptography must use established libraries.
Contributions use MPL-2.0; preserve upstream notices and attribution.
