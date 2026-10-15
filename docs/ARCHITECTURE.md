# Architecture

Quill keeps Gecko outside this repository. A Firefox-derived distribution is the
intended product: mature content sandboxes, navigation, accessibility, TLS,
WebExtensions, WebAuthn and platform integration should remain upstream-owned.
A custom webview shell would lose too much of that integration.

The current browser launcher runs an explicitly supplied Firefox binary in an
isolated development profile. A temporary WebExtension supplies Quill's new tab
and request URL cleaning; userChrome CSS trims visual noise. This is a development
host, not a rebranded binary. Firefox's window controls, settings, downloads,
history, bookmarks, password manager and extension manager remain real Firefox
features. If extension installation is blocked, the launcher supplies only a local homepage
and clearly reports the missing extension features. Quill's Rust vault is separate. No browser history is imported into search.

`privacy/`, `search/`, `vault/` and `updater/` are independent Rust components.
Only the crawler performs network I/O. The updater only verifies local bytes.
The browser extension shares the tracking parameter data with the Rust crate.
There is no local HTTP daemon, backend, Quill account, telemetry or cloud model.

For a distribution, pin a supported Firefox ESR commit outside this checkout,
apply reviewed patches, supply a complete branding directory, and build with
Mozilla's `mach`. `browser/mozconfig.example` is an integration starting point,
not a tested release recipe. Native component integration needs reviewed IPC,
private-session boundaries, engine network observers and packaging on every OS.
Do not disable the content sandbox or extension signature enforcement in releases.

References: [Mozilla build setup](https://firefox-source-docs.mozilla.org/setup/),
[mozconfig](https://firefox-source-docs.mozilla.org/build/buildsystem/mozconfigs.html),
[branding](https://firefox-source-docs.mozilla.org/browser/branding/docs/index.html).

## Licensing

Quill uses MPL-2.0, including the supplied brand assets. Dependencies keep their
licenses; inspect `Cargo.lock` and `cargo metadata` before distributing binaries.
Changed MPL files must remain available under MPL, including modified Gecko
files. Distribute corresponding source and notices with releases. Do not relabel
Mozilla source or use Firefox trademarks as Quill's product branding. See the
[Mozilla FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/). Codec, DRM and platform
redistribution requirements need separate release review; no codecs or DRM are
bundled here.
