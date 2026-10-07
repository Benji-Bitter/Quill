use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};

pub const SIGNATURE_DOMAIN: &[u8] = b"quill-update-manifest-v1\0";
const MAX_MANIFEST: usize = 16 * 1024;
const MAX_PACKAGE: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum UpdateMode {
    Automatic,
    Notify,
    #[default]
    Off,
}
impl UpdateMode {
    pub fn background_check_allowed(self) -> bool {
        self != Self::Off
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    pub sequence: u64,
    pub expires_unix: u64,
    pub target: String,
    pub package_bytes: u64,
    pub sha256: [u8; 32],
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown signing key")]
    UnknownKey,
    #[error("invalid manifest signature")]
    Signature,
    #[error("invalid manifest metadata")]
    Metadata,
    #[error("package length or digest mismatch")]
    Package,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Trusted keys must come from local release configuration, never the manifest.
pub fn verify_manifest(
    bytes: &[u8],
    signature: &[u8],
    key_id: &str,
    keys: &BTreeMap<String, [u8; 32]>,
    target: &str,
    minimum_sequence: u64,
    now_unix: u64,
) -> Result<Manifest, Error> {
    if bytes.len() > MAX_MANIFEST {
        return Err(Error::Metadata);
    }
    let key = keys.get(key_id).ok_or(Error::UnknownKey)?;
    let key = VerifyingKey::from_bytes(key).map_err(|_| Error::Signature)?;
    let signature = Signature::from_slice(signature).map_err(|_| Error::Signature)?;
    let mut message = SIGNATURE_DOMAIN.to_vec();
    message.extend_from_slice(bytes);
    key.verify_strict(&message, &signature)
        .map_err(|_| Error::Signature)?;
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| Error::Metadata)?;
    if manifest.schema != 1
        || manifest.version.is_empty()
        || manifest.version.len() > 64
        || manifest.target != target
        || manifest.sequence <= minimum_sequence
        || manifest.expires_unix <= now_unix
        || manifest.package_bytes == 0
        || manifest.package_bytes > MAX_PACKAGE
    {
        return Err(Error::Metadata);
    }
    Ok(manifest)
}

pub fn verify_package(manifest: &Manifest, mut package: impl Read) -> Result<(), Error> {
    if manifest.package_bytes == 0 || manifest.package_bytes > MAX_PACKAGE {
        return Err(Error::Metadata);
    }
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = package.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > manifest.package_bytes {
            return Err(Error::Package);
        }
        hash.update(&buffer[..n]);
    }
    let digest: [u8; 32] = hash.finalize().into();
    if count != manifest.package_bytes || digest != manifest.sha256 {
        return Err(Error::Package);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    fn fixture() -> (Manifest, SigningKey, BTreeMap<String, [u8; 32]>) {
        let signing = SigningKey::from_bytes(&[7; 32]); // Test-only deterministic key, never a release key.
        let keys = BTreeMap::from([("test".into(), signing.verifying_key().to_bytes())]);
        (
            Manifest {
                schema: 1,
                version: "0.1.0".into(),
                sequence: 2,
                expires_unix: 200,
                target: "test-target".into(),
                package_bytes: 7,
                sha256: Sha256::digest(b"package").into(),
            },
            signing,
            keys,
        )
    }
    fn sign(bytes: &[u8], key: &SigningKey) -> Vec<u8> {
        let mut message = SIGNATURE_DOMAIN.to_vec();
        message.extend_from_slice(bytes);
        key.sign(&message).to_bytes().to_vec()
    }
    #[test]
    fn valid_signature_and_package() {
        let (manifest, signing, keys) = fixture();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let verified = verify_manifest(
            &bytes,
            &sign(&bytes, &signing),
            "test",
            &keys,
            "test-target",
            1,
            100,
        )
        .unwrap();
        verify_package(&verified, &b"package"[..]).unwrap();
        for bad in [b"packag".as_slice(), b"package!", b"corrupt"] {
            assert!(verify_package(&verified, bad).is_err());
        }
    }
    #[test]
    fn rejects_untrusted_metadata() {
        let (manifest, signing, keys) = fixture();
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let signature = sign(&bytes, &signing);
        assert!(matches!(
            verify_manifest(&bytes, &signature, "unknown", &keys, "test-target", 1, 100),
            Err(Error::UnknownKey)
        ));
        assert!(verify_manifest(&bytes, &[0; 64], "test", &keys, "test-target", 1, 100).is_err());
        assert!(
            verify_manifest(
                &bytes[..bytes.len() - 1],
                &signature,
                "test",
                &keys,
                "test-target",
                1,
                100
            )
            .is_err()
        );
        for (target, sequence, now) in [
            ("wrong", 1, 100),
            ("test-target", 2, 100),
            ("test-target", 1, 200),
        ] {
            assert!(
                verify_manifest(&bytes, &signature, "test", &keys, target, sequence, now).is_err()
            );
        }
        let truncated = b"{\"schema\":1}";
        assert!(
            verify_manifest(
                truncated,
                &sign(truncated, &signing),
                "test",
                &keys,
                "test-target",
                1,
                100
            )
            .is_err()
        );
    }
    #[test]
    fn off_is_default_and_forbids_background_checks() {
        assert_eq!(UpdateMode::default(), UpdateMode::Off);
        assert!(!UpdateMode::Off.background_check_allowed());
        assert!(UpdateMode::Notify.background_check_allowed());
    }
}
