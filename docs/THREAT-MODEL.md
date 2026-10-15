# Threat model

Web content, extensions, fetched HTML, downloaded files, index files, update
metadata and prospective peers are hostile. Local administrator malware and a
compromised running OS can read an unlocked vault and are outside this prototype's
protection. A locked vault should resist offline guessing according to password
strength and Argon2id cost; authenticated encryption rejects ciphertext changes.

Gecko owns content isolation, TLS and the web platform. The development host uses
a temporary extension with broad navigation permissions; that privileged code is
part of the trusted base. It has no content scripts or external message handler,
uses a restrictive CSP, never evaluates page strings and builds results with
textContent. The launcher briefly exposes privileged loopback automation, then stops accepting
new connections. A malicious local process racing startup is not excluded; this
is one reason the launcher is not a production architecture.

Rust crates forbid unsafe source code. Dependencies may use audited unsafe code;
this rule does not claim a wholly safe dependency graph. HTML/index sizes, query
length, posting consistency and fetch deadlines bound resource use. HTML is parsed,
not rendered. The crawler accepts an explicit operator-chosen origin, including
loopback for tests. Never expose it as a URL-fetching service: DNS rebinding and
private-network access need stronger network isolation for unattended crawling.
A canonical hint or content hash does not prove ownership or truth.

Vault threats include stolen disks, offline password attacks, malicious files,
concurrent writers and unintended retention. Per-record active keys are removed
on deletion; previous copies are not revoked. See [vault](VAULT.md). Platform ACLs,
secure key storage and backup-aware erasure remain release blockers.

Update signatures defend against a compromised distribution mirror. They do not
defend against stolen signing keys, a compromised build pipeline, expired trust
roots, rollback of local sequence state or a compromised installer. The current
component only verifies bytes. See [updates](UPDATES.md).

A distributed index must assume Sybil peers, poisoned rankings, selective
withholding, shard tampering, query correlation and denial of service. Verify
content-addressed shards and signatures, limit CPU/memory/bandwidth per peer,
require diverse provenance, and avoid sharing raw queries. Peer signatures alone
do not establish independent corroboration. Quill does not claim anonymity.

## Private sessions and permissions

The extension is forbidden in private windows, so it cannot index or learn from
them. Private windows use Gecko's existing private mode, including its temporary
storage behavior. We have not independently verified every cache, service worker,
IndexedDB or OS persistence path. Downloads/bookmarks explicitly saved by the user
may persist. No promise is made about wiping RAM or OS caches.

Camera, microphone and location retain Gecko's ask-first behavior. Notifications,
MIDI and sensors are disabled in the development preferences. Bluetooth/USB are
not new capabilities introduced here. Clipboard remains governed by Gecko's
secure-context and interaction checks. The Rust permission configuration is a
model, not an additional enforcement layer. Allow-once and tab-scoped grants must
be mapped to actual engine capabilities; they are not invented UI options.

Tracking protection and storage partitioning use upstream Gecko features.
Quill's host-list classifier has no shipped ad list or engine hook. Comprehensive
ad/analytics/bounce tracking blocking, stronger fingerprint resistance and complete
WebRTC leak protection are not claimed. Aggressive tab sleeping is also not built;
only upstream low-memory unloading is enabled. A future policy must exclude calls,
media, downloads, active forms and pinned/whitelisted tabs and test restoration.
