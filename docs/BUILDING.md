# Building and running

Install Rust through rustup, a native C toolchain (Xcode Command Line Tools on
macOS, MSVC Build Tools on Windows, GCC/Clang on Linux), and Python 3.11+. Node.js 24 runs the extension unit tests.
The repository pins Rust 1.93.1. Cargo downloads locked dependencies during build;
these are build-time connections, not browser runtime traffic.

```sh
cargo build --workspace --release --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
node --test tests/extension.test.mjs
python3 -m unittest discover -s tests
python3 tools/search_demo.py
```

The CLI is `target/release/quill-search` (`.exe` on Windows). Paths passed to it
must have an existing parent directory. Indexes are private user data; keep them
outside Git. The demo saves to the ignored `.quill/demo-index.json`.

## Browser development host

Install a current Firefox release from Mozilla. Supply the real executable:

```sh
python3 tools/browser.py --firefox /Applications/Firefox.app/Contents/MacOS/firefox
# Linux example
python3 tools/browser.py --firefox /usr/bin/firefox
# Windows example
py tools/browser.py --firefox 'C:\Program Files\Mozilla Firefox\firefox.exe'
```

A separately downloaded engine can live under ignored `.quill/engine/`. No engine
binary is committed. Do not run old engines just because a prototype was tested
against them. Firefox on macOS is universal; other architectures need matching
upstream binaries. Native Rust builds target the host architecture. CI checks
Linux, Windows and macOS, not all six OS/architecture combinations.

The launcher creates `.quill/browser-profile`, installs a temporary local extension
through Firefox's loopback Marionette protocol with development system access, then stops accepting new automation
connections. It leaves extension signing and the content sandbox enabled. The
extension is temporary and must be reinstalled by the launcher on each start.
The launcher supervises the process and holds a separate launcher lock until exit.
This automation-based setup is for development; it is not release packaging.

If an administrator blocks extensions, installation may produce a generic Firefox
compatibility error. The launcher falls back to a local Quill homepage and reports
that local search and URL cleaning are unavailable. New tabs retain Firefox's native
page. No policy or signature check is bypassed. On this development Mac, the managed
policy permits only an approved extension list; the Quill extension is not on it.

With prerequisites installed, `python3 tools/bootstrap.py --firefox /path/to/firefox`
builds the native components and launches the host in one command. The Linux CI
helper `python3 tools/fetch_firefox_linux.py` downloads the recorded Mozilla engine
and checks its HTTPS-published checksum. This is development tooling, not the Quill
signed update architecture. `browser/engine-reference.json` records tested source
provenance. Use a supported current engine for subsequent development.

A profile marked as Quill-owned can be reused. Do not point the launcher at your
normal browser profile. Delete the development profile while Firefox is closed to
remove its browsing data. The installed Firefox engine retains upstream networking
and its own updater; Quill's update verifier is separate and does not connect.

```sh
python3 tools/browser_smoke.py --firefox /path/to/firefox
# Explicitly include a real HTTPS request to example.com:
python3 tools/browser_smoke.py --firefox /path/to/firefox --https
```

The smoke test creates a temporary profile and loopback fixture. Screenshots and
measurements go to `.quill/measurements`. The smoke profile is removed after exit.

## Building a distribution

No one-command Gecko source build is claimed yet. Use Mozilla's [platform setup
instructions](https://firefox-source-docs.mozilla.org/setup/) in a separate checkout.
Pin a supported release commit, use a Quill-specific patch series, implement full
branding/build integration, then run `mach build`, `mach test` and `mach package`.
`browser/mozconfig.example` is only a starting point. A complete source build needs
substantial disk space, memory and time; this workspace did not build Gecko.

A release additionally needs reproducible source provenance, platform signing and
notarization, installer testing, update keys, source archives and license notices.
