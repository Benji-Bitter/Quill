# Network transparency

Current Quill-owned network behavior is simple: the search crawler fetches an
explicit seed origin and its robots file. The Rust index, privacy, vault and
update verifier have no networking code. The development extension has no fetch,
XHR, WebSocket, native messaging or telemetry client. URL cleaning may redirect
a navigation to the same site without known tracking parameters.

This does **not** establish that Firefox is silent. The host retains upstream
services including certificate/security checks, safe browsing, extension services
and engine update checks. Preferences disable telemetry/studies/suggestions and
speculative traffic where known, but preference names are version-dependent and
not a substitute for an engine audit. Websites and operating systems also connect.

Use `python3 tools/network_snapshot.py --pid PID` on macOS/Linux to inspect current
sockets for the browser and its child processes. It invokes `lsof -nP`, avoiding
reverse-DNS lookups. A snapshot misses short-lived connections, DNS requests and
traffic delegated to OS daemons; an empty snapshot is not a zero-byte proof.
For an acceptance audit, capture packets in an isolated test account/VM, start
from a clean profile, load no webpage, disable Quill updates, and compare a full
startup plus idle trace with process attribution. Do not commit real user traces.

A future engine event stream should carry monotonic time, origin attributes,
private-session flag, request initiator, top-level site, destination, transport,
byte counts and policy decision. Classify first/third party using the public
suffix list and schemeful site rules, not string suffixes. Separately label
tracker/ad/analytics matches with rule provenance. Cover HTTP channels, WebSocket,
WebRTC ICE/media, observable DNS, extensions and browser services. Keep a bounded
in-memory buffer by default, clear private records with the session, and export
only on explicit action. Content hooks alone cannot observe all these sources.

Locally imported blocklists need validation, size limits, license provenance and
atomic replacement. No remote list subscription is implemented. Users should be
able to inspect and block a connection with a clear warning when doing so disables
a security service or breaks a website.

## Observed development host

On 15 October 2026, a socket snapshot of the visible Firefox development host
with only the local Quill homepage loaded showed multiple established remote TLS
connections and UDP sockets. No automation listening socket remained. This confirms
that the host is not network-silent. A socket snapshot does not identify the
initiating service or prove destination ownership; attributing those connections
requires an engine event trace and packet capture. Raw addresses and local user
information are intentionally not published in this repository.
