use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const LIMIT: u64 = 8 * 1024 * 1024;
const AAD: &[u8] = b"quill-vault-v1";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("vault authentication failed")]
    Authentication,
    #[error("invalid or oversized vault")]
    Invalid,
    #[error("vault is locked by another process (or a stale lock)")]
    Locked,
    #[error("record not found")]
    Missing,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    salt: [u8; 16],
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Record {
    key: [u8; 32],
    nonce: [u8; 12],
    ciphertext: Vec<u8>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    entries: BTreeMap<String, Record>,
}

struct Lock(PathBuf);
impl Lock {
    fn acquire(path: &Path) -> Result<Self> {
        let mut name = path.as_os_str().to_owned();
        name.push(".lock");
        let path = PathBuf::from(name);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Error::Locked
                } else {
                    Error::Io(e)
                }
            })?;
        Ok(Self(path))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub struct Vault {
    path: PathBuf,
    salt: [u8; 16],
    key: Zeroizing<[u8; 32]>,
    records: Records,
    _lock: Lock,
}
fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| Error::Invalid)?;
    Ok(bytes)
}
fn derive(password: &[u8], salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>> {
    if password.is_empty() || password.len() > 1024 {
        return Err(Error::Invalid);
    }
    let params = Params::new(64 * 1024, 3, 1, Some(32)).map_err(|_| Error::Invalid)?;
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|_| Error::Invalid)?;
    Ok(key)
}
fn seal(key: &[u8], nonce: &[u8; 12], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| Error::Invalid)?
        .encrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| Error::Authentication)
}
fn unseal(
    key: &[u8],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| Error::Invalid)?
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Authentication)
}
fn read(path: &Path) -> Result<Envelope> {
    let mut bytes = Vec::new();
    File::open(path)?.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(Error::Invalid);
    }
    let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
    if envelope.version != 1 {
        return Err(Error::Invalid);
    }
    Ok(envelope)
}
impl Vault {
    pub fn create(path: &Path, password: &[u8]) -> Result<Self> {
        let lock = Lock::acquire(path)?;
        if path.exists() {
            return Err(Error::Invalid);
        }
        let salt = random()?;
        let vault = Self {
            path: path.into(),
            salt,
            key: derive(password, &salt)?,
            records: Records::default(),
            _lock: lock,
        };
        vault.save()?;
        Ok(vault)
    }
    pub fn open(path: &Path, password: &[u8]) -> Result<Self> {
        let lock = Lock::acquire(path)?;
        let envelope = read(path)?;
        let key = derive(password, &envelope.salt)?;
        let plaintext = unseal(key.as_ref(), &envelope.nonce, &envelope.ciphertext, AAD)?;
        let records: Records = serde_json::from_slice(&plaintext).map_err(|_| Error::Invalid)?;
        if records.entries.len() > 1000 {
            return Err(Error::Invalid);
        }
        Ok(Self {
            path: path.into(),
            salt: envelope.salt,
            key,
            records,
            _lock: lock,
        })
    }
    pub fn insert(&mut self, secret: &[u8]) -> Result<String> {
        if secret.len() > 64 * 1024 || self.records.entries.len() >= 1000 {
            return Err(Error::Invalid);
        }
        let id = random::<16>()?
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let key = Zeroizing::new(random::<32>()?);
        let nonce = random()?;
        let ciphertext = seal(key.as_ref(), &nonce, secret, id.as_bytes())?;
        self.records.entries.insert(
            id.clone(),
            Record {
                key: *key,
                nonce,
                ciphertext,
            },
        );
        if let Err(error) = self.save() {
            self.records.entries.remove(&id);
            return Err(error);
        }
        Ok(id)
    }
    pub fn get(&self, id: &str) -> Result<Zeroizing<Vec<u8>>> {
        let record = self.records.entries.get(id).ok_or(Error::Missing)?;
        unseal(
            &record.key,
            &record.nonce,
            &record.ciphertext,
            id.as_bytes(),
        )
    }
    pub fn ids(&self) -> Vec<String> {
        self.records.entries.keys().cloned().collect()
    }
    /// The UI must collect final confirmation before invoking this operation.
    /// Historical copies of the vault remain decryptable with their password.
    pub fn delete(&mut self, id: &str, password: &[u8]) -> Result<()> {
        let envelope = read(&self.path)?;
        let old_key = derive(password, &envelope.salt)?;
        let _authenticated = unseal(old_key.as_ref(), &envelope.nonce, &envelope.ciphertext, AAD)?;
        let salt = random()?;
        let key = derive(password, &salt)?;
        let record = self.records.entries.remove(id).ok_or(Error::Missing)?;
        let old_salt = std::mem::replace(&mut self.salt, salt);
        let old_key = std::mem::replace(&mut self.key, key);
        if let Err(error) = self.save() {
            self.records.entries.insert(id.into(), record);
            self.salt = old_salt;
            self.key = old_key;
            return Err(error);
        }
        // Record::drop zeroizes the active per-record key and ciphertext buffer.
        drop(record);
        Ok(())
    }
    fn save(&self) -> Result<()> {
        let plaintext =
            Zeroizing::new(serde_json::to_vec(&self.records).map_err(|_| Error::Invalid)?);
        let nonce = random()?;
        let envelope = Envelope {
            version: 1,
            salt: self.salt,
            nonce,
            ciphertext: seal(self.key.as_ref(), &nonce, &plaintext, AAD)?,
        };
        let bytes = serde_json::to_vec(&envelope).map_err(|_| Error::Invalid)?;
        if bytes.len() as u64 > LIMIT {
            return Err(Error::Invalid);
        }
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistent_authentication_tamper_and_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.vault");
        let mut vault = Vault::create(&path, b"test password only").unwrap();
        let id = vault.insert(b"fictional credential").unwrap();
        let second = vault.insert(b"other fictional credential").unwrap();
        assert!(matches!(
            Vault::open(&path, b"test password only"),
            Err(Error::Locked)
        ));
        drop(vault);
        assert!(matches!(
            Vault::open(&path, b"wrong password"),
            Err(Error::Authentication)
        ));
        let mut vault = Vault::open(&path, b"test password only").unwrap();
        assert_eq!(&**vault.get(&id).unwrap(), b"fictional credential");
        assert!(vault.delete(&id, b"wrong password").is_err());
        vault.delete(&id, b"test password only").unwrap();
        assert!(!vault.records.entries.contains_key(&id));
        assert!(matches!(vault.get(&id), Err(Error::Missing)));
        drop(vault);
        let vault = Vault::open(&path, b"test password only").unwrap();
        assert!(vault.get(&id).is_err());
        assert_eq!(
            &**vault.get(&second).unwrap(),
            b"other fictional credential"
        );
        drop(vault);
        let mut envelope = read(&path).unwrap();
        envelope.ciphertext[0] ^= 1;
        fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(matches!(
            Vault::open(&path, b"test password only"),
            Err(Error::Authentication)
        ));
    }
    #[test]
    fn record_key_and_id_bind_ciphertext() {
        let key = [3; 32];
        let nonce = [4; 12];
        let ciphertext = seal(&key, &nonce, b"fixture", b"record-a").unwrap();
        assert!(unseal(&[0; 32], &nonce, &ciphertext, b"record-a").is_err());
        assert!(unseal(&key, &nonce, &ciphertext, b"record-b").is_err());
    }
}
