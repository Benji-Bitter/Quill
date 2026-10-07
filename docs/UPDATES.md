# Updates

`quill-updater` verifies local bytes only. No update HTTP client, scheduler,
installer, release signing key or production release package exists. Off is the
default mode and disallows background checks; there is no code that contacts
GitHub. Firefox's own engine updater is separate in the development host.

A manifest carries a schema, version label, monotonically increasing sequence,
expiry timestamp, exact target, package byte count and SHA-256 digest. An Ed25519
signature covers a fixed Quill domain separator followed by the exact manifest
bytes. The verifier looks up a caller-supplied key ID in a locally trusted keyring;
keys from a manifest or mirror must never populate that keyring automatically.
It authenticates before deserializing. Unknown keys, malformed JSON, altered or
truncated bytes, wrong platform, stale sequence, expiry, overlarge metadata,
incorrect package length and digest all fail closed.

The verifier bounds manifests to 16 KiB and packages to 2 GiB. The package hash is
streamed. A future installer must keep the verified file handle or otherwise bind
installation to exactly the verified bytes, reject archive path traversal and
links, verify platform signatures, stage atomically and persist rollback protection.
The current verifier does not safely unpack or install anything.

Automatic and Notify modes only express policy today. A future scheduler may use
GitHub Releases as its default and only project-controlled endpoint. Off means no
request. Check Now is an explicit one-shot action; local-file verification needs
no network. Advanced users can supply a mirror, local file, LAN source or custom
HTTPS URL, but cannot bypass signature checks. No machine identifier, account,
browsing information or search query belongs in any request.

Release keys should be generated and held offline or in hardware. Use separate
roles for release signing and root rotation, document expiration/revocation, and
ship a rehearsed compromise-recovery procedure. Store accepted sequence state
outside an untrusted download directory. Local state rollback and clock rollback
remain threats. All keys in unit tests are deterministic test-only fixtures.
There is deliberately no apparently usable default production key.
