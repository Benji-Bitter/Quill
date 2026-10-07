# Privacy principles

Quill must work without an account or the project's continued existence. There
is no Quill sync service, analytics, telemetry, advertising or remote AI. Browsing
history and local preferences must never be uploaded to Quill. Public search
must not depend on a company-owned query service or paid organic ranking.

User data belongs on the device unless the user deliberately exports or sends
it. Features must provide inspectable storage, export and deletion. Local
learning, if added, must be opt-in, explainable and resettable. Neither local
learning nor private browsing data may enter a distributed index.

Privacy behavior and ranking must be inspectable source code. A privacy claim
is a testable engineering promise, not a slogan. The current browser host still
contains upstream networking and has not passed a zero-background-traffic audit.

GitHub Releases may eventually distribute optional Quill updates. Off must mean
no request, including metadata, DNS or connectivity probes. Automatic and Notify
modes require explicit choice. The present verifier has no network client and
defaults to Off. No Quill server, identifier or search data is involved.

Websites necessarily receive the requests users make. DNS, operating-system
services, certificate verification and upstream engine services are separate
from project-controlled traffic and must be disclosed and measured. Disabling
security mechanisms merely to make a traffic counter smaller is not acceptable.

Users must be able to remove their data. SSD wear levelling, filesystem snapshots,
backups and OS paging prevent promises of physical overwriting. See the vault
threat model for the current deletion limitations.
