# Experimental vault

`quill-vault` is a local library, not Firefox's password manager or an autofill
implementation. Do not store real credentials until it has had independent review.
There is no passkey store, platform secure storage, recovery UI or sync service.

The format uses RustCrypto Argon2id (64 MiB, three iterations, one lane) to derive
a 256-bit wrapping key from the supplied password and a random 128-bit salt.
AES-256-GCM authenticates an encrypted record map with a random 96-bit nonce.
Each record has a separate random 256-bit key and nonce, with its ID authenticated
as associated data. Keys live inside the encrypted map; the outer format and
record format have separate authenticated contexts. No custom cipher is used.
The file parser limits total size to 8 MiB and uses fixed KDF costs rather than
trusting expensive parameters supplied in an attacker-controlled file.

Operations take an exclusive create-new lock, write a same-directory temporary
file, sync it, and replace the active file. A failed insert/delete restores the
in-memory state. A crash may leave a lock file; remove it only after verifying no
process owns the vault. Unix temporary files have owner-only permissions. Windows
inherits the directory ACL: use a private directory. Directory-entry durability
across sudden power loss and hostile-directory attacks require further work.

Passwords are passed as bytes and never logged. Returned secrets use zeroizing
buffers; active record keys and plaintext serialization buffers are zeroized on
drop. This does not promise wiping all compiler temporaries, RAM, swap, crash dumps
or OS caches. Callers own password buffer lifetime. Disk encryption remains useful.

## Deletion and recovery

The library reauthenticates against the stored vault before deleting a record.
It removes the record and its key slot, derives a fresh outer key with a fresh
salt, writes a new authenticated vault, and zeroizes the active removed key.
The tests verify close/reopen, wrong-password rejection, tamper rejection and
absence of deleted records through normal application access.

This destroys the active application's record key, **not historical copies**.
Because previous key slots were wrapped with a password-derived key, someone with
an old file snapshot and the password can recover the old record. Rotation does
not revoke that copy. Stronger cryptographic erasure requires per-record secrets
in platform-backed storage with explicit snapshot/recovery semantics. That is a
release blocker, not a property claimed by this prototype. SSD wear levelling,
journals and backups prevent honest promises of arbitrary physical overwrites.

The future UI should use Delete saved login → Continue → password/platform
verification → Permanently delete. Final confirmation calls the destructive API.
It must also remove autofill/search references and caches; none are connected yet.

A closed vault file is an encrypted backup/export today. Copying it retains all
included credentials and key slots. There is no password reset or recovery backdoor;
loss of the password loses the vault. A future recovery key is opt-in, offline,
separately wrapped, and explicitly subject to deletion/backup limitations. Plaintext
export must require deliberate verification and warn that the output is unencrypted.
WebAuthn should continue through Gecko and OS authenticators, not invented crypto.
