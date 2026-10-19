//! APK Signature Scheme v2, which every Android Nativella supports checks
//! (API level 24 and later): an APK Signing Block between the entries and
//! the central directory, which signs the digest of all the rest.
//!
//! See https://source.android.com/docs/security/features/apksigning/v2. The
//! key is ECDSA on P-256, with SHA-256; its certificate is self-signed and
//! made from the key alone, so that the same key always signs with the same
//! certificate: Android installs an update only from the same one.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use p256::ecdsa::signature::Signer;
use p256::ecdsa::{DerSignature, SigningKey};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use sha2::{Digest, Sha256};

use super::zip::{self, Unfinished};

const BLOCK_MAGIC: &[u8; 16] = b"APK Sig Block 42";
const V2_BLOCK_ID: u32 = 0x7109_871a;
/// ECDSA with SHA2-256, the content digested in chunks of 1 MiB.
const ECDSA_WITH_SHA256: u32 = 0x0201;
const CHUNK_LENGTH: usize = 1024 * 1024;

/// A signing key, its certificate, and the file it comes from.
pub struct Key {
    signing_key: SigningKey,
    certificate: Vec<u8>,
    path: PathBuf,
}

impl Key {
    /// The key of a PKCS #8 PEM file, as `openssl genpkey -algorithm EC
    /// -pkeyopt ec_paramgen_curve:P-256` writes it.
    pub fn load(path: &Path) -> io::Result<Self> {
        let pem = fs::read_to_string(path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Cannot read the key {}: {error}.", path.display()),
            )
        })?;
        let signing_key = SigningKey::from_pkcs8_pem(&pem).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} is not an EC P-256 private key in PKCS #8 PEM: {error}.",
                    path.display()
                ),
            )
        })?;
        let certificate = certificate(&signing_key)?;

        Ok(Self {
            signing_key,
            certificate,
            path: path.to_owned(),
        })
    }

    /// The debug key of this computer, made at its first use: enough to try
    /// an application, not to publish it, as the key could be lost.
    pub fn debug() -> io::Result<Self> {
        let path = config_dir()?
            .join("nativella")
            .join("android-debug-key.pem");
        if !path.is_file() {
            create_debug_key(&path)?;
        }

        Self::load(&path)
    }

    /// The file of the key.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Writes a new key to `path`, whole, unless another run of `nativella` has
/// just written one.
fn create_debug_key(path: &Path) -> io::Result<()> {
    let signing_key = loop {
        let mut secret = [0; 32];
        getrandom::fill(&mut secret).map_err(io::Error::other)?;
        // A scalar of the curve: all but 1 in 2^32 random numbers are.
        if let Ok(signing_key) = SigningKey::from_slice(&secret) {
            break signing_key;
        }
    };
    let pem = signing_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(io::Error::other)?;

    fs::create_dir_all(path.parent().expect("A file in a directory."))?;
    // Written aside, then linked in place: a run killed halfway leaves no
    // truncated key, and the link fails if another run linked its own first.
    let staging = path.with_extension(format!("pem.tmp-{}", std::process::id()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&staging)?;
    file.write_all(pem.as_bytes())?;
    file.sync_all()?;
    drop(file);

    let linked = fs::hard_link(&staging, path);
    fs::remove_file(&staging)?;
    match linked {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => Err(error),
        _ => Ok(()),
    }
}

/// Signs the archive `file`, whose entries `unfinished` describes: writes the
/// APK Signing Block after them, then the central directory and its end.
pub fn sign(file: &mut File, unfinished: &Unfinished, key: &Key) -> io::Result<()> {
    // The end of central directory as it is without the block, which goes
    // where the central directory starts.
    let end = zip::end_of_central_directory(unfinished, unfinished.entries_length)?;
    let digest = digest(
        file,
        unfinished.entries_length,
        &unfinished.central_directory,
        &end,
    )?;
    let block = signing_block(&digest, key)?;

    file.seek(SeekFrom::Start(unfinished.entries_length))?;
    file.write_all(&block)?;
    file.write_all(&unfinished.central_directory)?;
    file.write_all(&zip::end_of_central_directory(
        unfinished,
        unfinished.entries_length + block.len() as u64,
    )?)?;

    file.flush()
}

/// The digest of the entries, the central directory and its end: of each
/// chunk of 1 MiB, then of those digests.
fn digest(
    file: &mut File,
    entries_length: u64,
    central_directory: &[u8],
    end: &[u8],
) -> io::Result<[u8; 32]> {
    let mut chunk_digests = Vec::new();
    let mut chunk = vec![0; CHUNK_LENGTH];
    let mut digest_chunk = |data: &[u8]| {
        let mut hasher = Sha256::new();
        hasher.update([0xa5]);
        hasher.update((data.len() as u32).to_le_bytes());
        hasher.update(data);
        chunk_digests.extend(hasher.finalize());
    };

    file.seek(SeekFrom::Start(0))?;
    let mut remaining = entries_length;
    while remaining > 0 {
        let length = remaining.min(CHUNK_LENGTH as u64) as usize;
        file.read_exact(&mut chunk[..length])?;
        digest_chunk(&chunk[..length]);
        remaining -= length as u64;
    }
    for section in [central_directory, end] {
        for data in section.chunks(CHUNK_LENGTH) {
            digest_chunk(data);
        }
    }

    let count = chunk_digests.len() / 32;
    let mut hasher = Sha256::new();
    hasher.update([0x5a]);
    hasher.update((count as u32).to_le_bytes());
    hasher.update(&chunk_digests);

    Ok(hasher.finalize().into())
}

/// The APK Signing Block, holding the v2 signature of `digest` by `key`.
fn signing_block(digest: &[u8], key: &Key) -> io::Result<Vec<u8>> {
    let mut digest_entry = ECDSA_WITH_SHA256.to_le_bytes().to_vec();
    digest_entry.extend(prefixed(digest));

    let mut signed_data = prefixed(&prefixed(&digest_entry));
    signed_data.extend(prefixed(&prefixed(&key.certificate)));
    // No additional attribute.
    signed_data.extend(prefixed(&[]));

    let signature: DerSignature = key.signing_key.sign(&signed_data);
    let mut signature_entry = ECDSA_WITH_SHA256.to_le_bytes().to_vec();
    signature_entry.extend(prefixed(signature.as_bytes()));
    let public_key = key
        .signing_key
        .verifying_key()
        .to_public_key_der()
        .map_err(io::Error::other)?;

    let mut signer = prefixed(&signed_data);
    signer.extend(prefixed(&prefixed(&signature_entry)));
    signer.extend(prefixed(public_key.as_bytes()));
    let signers = prefixed(&prefixed(&signer));

    // One ID-value pair, its length on 8 bytes.
    let mut pairs = ((4 + signers.len()) as u64).to_le_bytes().to_vec();
    pairs.extend(V2_BLOCK_ID.to_le_bytes());
    pairs.extend(signers);

    // Its size, without the first size field.
    let size = (pairs.len() + 8 + BLOCK_MAGIC.len()) as u64;
    let mut block = size.to_le_bytes().to_vec();
    block.extend(pairs);
    block.extend(size.to_le_bytes());
    block.extend(BLOCK_MAGIC);

    Ok(block)
}

/// `data` after its length, on 4 bytes.
fn prefixed(data: &[u8]) -> Vec<u8> {
    let mut prefixed = (data.len() as u32).to_le_bytes().to_vec();
    prefixed.extend(data);
    prefixed
}

/// A self-signed X.509 certificate of `signing_key`, its fields fixed: the
/// same key always makes the same certificate (ECDSA signatures of p256 are
/// deterministic, RFC 6979).
fn certificate(signing_key: &SigningKey) -> io::Result<Vec<u8>> {
    // ecdsa-with-SHA256
    let algorithm = der(
        0x30,
        &der(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02]),
    );
    // CN=Nativella
    let name = der(
        0x30,
        &der(
            0x31,
            &der(
                0x30,
                &[der(0x06, &[0x55, 0x04, 0x03]), der(0x0c, b"Nativella")].concat(),
            ),
        ),
    );
    // From 2025 to 2075: Google Play asks for keys valid until 2033 at least.
    let validity = der(
        0x30,
        &[der(0x17, b"250101000000Z"), der(0x18, b"20750101000000Z")].concat(),
    );
    let public_key = signing_key
        .verifying_key()
        .to_public_key_der()
        .map_err(io::Error::other)?;

    let to_be_signed = der(
        0x30,
        &[
            // Version 3
            der(0xa0, &der(0x02, &[0x02])),
            // Serial number
            der(0x02, &[0x01]),
            algorithm.clone(),
            name.clone(),
            validity,
            name,
            public_key.as_bytes().to_vec(),
        ]
        .concat(),
    );
    let signature: DerSignature = signing_key.sign(&to_be_signed);
    // A bit string, without unused bits.
    let signature = der(0x03, &[&[0][..], signature.as_bytes()].concat());

    Ok(der(0x30, &[to_be_signed, algorithm, signature].concat()))
}

/// A DER value: its tag, its length, its content.
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut value = vec![tag];
    let length = content.len();
    if length < 0x80 {
        value.push(length as u8);
    } else {
        let bytes = length.to_be_bytes();
        let significant = &bytes[bytes
            .iter()
            .position(|&byte| byte != 0)
            .unwrap_or(bytes.len() - 1)..];
        value.push(0x80 | significant.len() as u8);
        value.extend(significant);
    }
    value.extend(content);
    value
}

/// The per-user configuration directory of the platform.
fn config_dir() -> io::Result<PathBuf> {
    let from_env = |name: &str| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let missing = || io::Error::new(io::ErrorKind::NotFound, "The user has no home directory.");

    if cfg!(windows) {
        from_env("APPDATA").ok_or_else(missing)
    } else if cfg!(target_os = "macos") {
        from_env("HOME")
            .map(|home| home.join("Library/Application Support"))
            .ok_or_else(missing)
    } else {
        from_env("XDG_CONFIG_HOME")
            .or_else(|| from_env("HOME").map(|home| home.join(".config")))
            .ok_or_else(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Android updates an application only if the new APK has the same
    /// certificate, byte for byte: any change to the certificate (its dates,
    /// its name, an extension, the encoding of the key) would prevent every
    /// application signed with a key of its author from updating.
    #[test]
    fn the_certificate_of_a_key_never_changes() {
        let signing_key = SigningKey::from_slice(&[7; 32]).unwrap();

        let certificate = certificate(&signing_key).unwrap();

        let digest: String = Sha256::digest(&certificate)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            digest,
            "a714a00fe2b9ba5828ed18f0896c991ef44ff946eb0ef7be7a8bd688f20aeb3a"
        );
    }

    #[test]
    fn the_debug_key_is_made_once() {
        let dir = Path::new(env!("OUT_DIR")).join(format!("key-test-{}", std::process::id()));
        let path = dir.join("android-debug-key.pem");

        create_debug_key(&path).unwrap();
        let first = fs::read(&path).unwrap();
        create_debug_key(&path).unwrap();

        assert_eq!(fs::read(&path).unwrap(), first, "The key in place stays.");
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            1,
            "Nothing is left aside."
        );
        Key::load(&path).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn der_lengths() {
        assert_eq!(der(0x04, &[1; 3])[..2], [0x04, 3]);
        assert_eq!(der(0x04, &[1; 200])[..3], [0x04, 0x81, 200]);
        assert_eq!(der(0x04, &[1; 300])[..4], [0x04, 0x82, 0x01, 0x2c]);
    }
}
