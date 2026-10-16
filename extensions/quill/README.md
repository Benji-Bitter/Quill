# Quill extensions

These original starter packages are licensed under the repository's MPL-2.0 license.
They run in Quill's native macOS browser, independently of its crawler and search index.

Open the puzzle-piece toolbar button, select a starter or paste a raw GitHub JSON
URL, preview its description and permissions, then install. Installed packages are
stored locally and work offline. Use Update to fetch and preview a replacement;
there are no automatic network updates. Enable/disable or remove packages here.
Reload an already open page after changes, especially when removing styles.

- **Quill Ad Blocker:** a small hand-written list covering 12 common ad-network
  domains. It blocks matching subresource requests, not top-level navigation.
  It is not EasyList/uBlock Origin and does not promise to block all ads, trackers,
  or YouTube video ads. Disable it if a website stops working.
- **Dark Pages:** simple dark CSS for reading; not a full adaptive dark-mode engine.
- **Reduced Motion:** removes CSS animations, transitions and smooth scrolling;
  it does not stop video playback or JavaScript-driven animation.

## Package format

A package is a UTF-8 JSON object:

```json
{
  "id": "my-style",
  "name": "My Style",
  "version": "1.0.0",
  "description": "Makes links easier to read.",
  "css": "a { text-decoration: underline !important; }"
}
```

`id` must match `[a-z0-9][a-z0-9-]{0,63}`. Supply `css`, `rules`, or both.
`rules` is a string containing a WebKit content-rule JSON array; only `block`
actions are supported. WebKit compiles and validates it before installation.
Styles apply to HTTP(S) top-level pages through a generated isolated-world style
injection. Styles may change or hide page content. CSS resource-loading constructs,
at-rules, slashes and escapes are rejected. Arbitrary extension JavaScript, native
code, background processes, cookie APIs and Chrome/Firefox manifests are unsupported.

Host the package at `https://raw.githubusercontent.com/OWNER/REPO/REF/path.json`.
Use a commit SHA as REF to pin a particular revision. Redirects are refused,
downloads are capped at 1 MB and 30 seconds, and downloads use an ephemeral HTTP
session with no website cookies. The manager shows the exact source and a SHA-256
checksum; this is provenance, not an author signature or trust certification.
An install with an existing ID replaces that package only after the user selects
Replace installed version. At most 20 packages / 8 MB can be stored locally.

The original `browser/extension` directory is the separate experimental Gecko
host's extension, not this package format.
