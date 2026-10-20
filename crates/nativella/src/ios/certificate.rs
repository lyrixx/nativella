//! The certificate of the developer, which Apple issues to sign apps, and
//! its private key: from a PKCS #12 file (`.p12`), as Keychain Access
//! exports it, or as OpenSSL makes it of the key and the certificate that the
//! Apple Developer website gives for it.
//!
//! The CMS signature of a code directory holds the certificate, with the
//! chain of Apple's that issued it, which a device needs to trust it: the
//! intermediate certificates of developers (Worldwide Developer Relations),
//! and their root. Nativella carries them, from
//! <https://www.apple.com/certificateauthority/>, for a file that does not.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, KeyIvInit};
use cms::builder::{SignedDataBuilder, SignerInfoBuilder, create_signing_time_attribute};
use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::ContentInfo;
use cms::encrypted_data::EncryptedData;
use cms::signed_data::{EncapsulatedContentInfo, SignerIdentifier};
use der::asn1::{ContextSpecific, ObjectIdentifier, OctetString, SetOfVec};
use der::{Any, Decode, Encode, Tag, Tagged};
use hmac::{Hmac, Mac};
use pkcs12::kdf::{self, Pkcs12KeyType};
use pkcs12::pbe_params::{EncryptedPrivateKeyInfo, Pkcs12PbeParams};
use pkcs12::pfx::Pfx;
use pkcs12::safe_bag::SafeContents;
use pkcs12::{cert_type::CertBag, mac_data::MacData};
use rsa::RsaPrivateKey;
use rsa::pkcs1v15::{Signature, SigningKey};
use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey};
use rsa::sha2::{Digest, Sha256};
use rsa::traits::PublicKeyParts;
use sha1::Sha1;
use x509_cert::attr::Attribute;
use x509_cert::name::Name;
use x509_cert::spki::AlgorithmIdentifierOwned;

const ID_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.1");
const ID_ENCRYPTED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.6");
const ID_SHA1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.14.3.2.26");
const ID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
const ID_X509_CERTIFICATE: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.22.1");
const ID_COMMON_NAME: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");
const ID_ORGANIZATIONAL_UNIT: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.11");
const ID_PBES2: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.5.13");
/// The certificates of developers: "iPhone Developer", which Apple
/// Development certificates have, and "iPhone Distribution", which Apple
/// Distribution ones have.
const APPLE_DEVELOPMENT: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113635.100.6.1.2");
const APPLE_DISTRIBUTION: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113635.100.6.1.4");
/// The attributes of the CMS signature of code signatures: the property list
/// of the hashes of the code directories, truncated, then the hashes.
const APPLE_CDHASHES: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113635.100.9.1");
const APPLE_CDHASHES2: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113635.100.9.2");
/// What a code directory hash is truncated to, in the property list.
const CDHASH_LENGTH: usize = 20;

/// Apple's certificates that issue those of developers: the Worldwide
/// Developer Relations ones that sign code, and their roots.
const APPLE_CERTIFICATES: [&[u8]; 6] = [
    include_bytes!("apple/AppleWWDRCAG3.cer"),
    include_bytes!("apple/AppleWWDRCAG4.cer"),
    include_bytes!("apple/AppleWWDRCAG5.cer"),
    include_bytes!("apple/AppleWWDRCAG6.cer"),
    include_bytes!("apple/AppleIncRootCertificate.cer"),
    include_bytes!("apple/AppleRootCA-G3.cer"),
];

/// A certificate of the developer, with its private key and its chain.
pub struct Certificate {
    path: PathBuf,
    key: SigningKey<Sha256>,
    /// The size of the signatures of the key, that of its modulus.
    signature_length: usize,
    certificate: x509_cert::Certificate,
    /// Its issuers, up to the root, as far as they are known.
    chain: Vec<x509_cert::Certificate>,
    /// The length of the certificate and of its chain, in DER, which the CMS
    /// signatures hold.
    certificates_length: usize,
    common_name: String,
    team: String,
}

impl Certificate {
    /// The certificate in the PKCS #12 file `path`, which `password`
    /// decrypts, and its private key.
    pub fn load(path: &Path, password: &str) -> io::Result<Self> {
        let data = fs::read(path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Cannot read the certificate {}: {error}.", path.display()),
            )
        })?;
        let contents = Contents::of(&data, password).map_err(|error| {
            io::Error::new(error.kind(), format!("{}: {error}", path.display()))
        })?;

        // The certificate of the key: the one with its public key.
        let mut identity = None;
        for key in &contents.keys {
            let key = RsaPrivateKey::from_pkcs8_der(key).map_err(|_| {
                invalid(&format!(
                    "The private key of {} is not an RSA key, which Apple's certificates of developers have.",
                    path.display()
                ))
            })?;
            let public = key
                .to_public_key()
                .to_public_key_der()
                .map_err(io::Error::other)?;
            if let Some(certificate) = contents.certificates.iter().find(|certificate| {
                certificate
                    .tbs_certificate
                    .subject_public_key_info
                    .to_der()
                    .is_ok_and(|info| info == public.as_bytes())
            }) {
                identity = Some((key, certificate.clone()));
                break;
            }
        }
        let (key, certificate) = identity.ok_or_else(|| {
            invalid(&format!(
                "{} holds no certificate with its private key: export the certificate with its key.",
                path.display()
            ))
        })?;

        let tbs = &certificate.tbs_certificate;
        let is_developer =
            tbs.extensions.iter().flatten().any(|extension| {
                [APPLE_DEVELOPMENT, APPLE_DISTRIBUTION].contains(&extension.extn_id)
            });
        if !is_developer {
            return Err(invalid(&format!(
                "{} is not an Apple Development or Apple Distribution certificate, which signs iOS apps.",
                path.display()
            )));
        }
        let expiry = tbs.validity.not_after;
        if expiry.to_system_time() < SystemTime::now() {
            return Err(invalid(&format!(
                "The certificate {} expired on {}: make another one on the Apple Developer website.",
                path.display(),
                expiry.to_date_time()
            )));
        }
        let common_name = attribute(&tbs.subject, ID_COMMON_NAME)
            .ok_or_else(|| invalid("The certificate has no common name."))?;
        // Apple puts the team of the developer there.
        let team = attribute(&tbs.subject, ID_ORGANIZATIONAL_UNIT)
            .ok_or_else(|| invalid("The certificate names no team (its organizational unit)."))?;

        let chain = chain(&certificate, &contents.certificates)?;
        if chain.is_empty() {
            return Err(invalid(&format!(
                "The certificate {} is issued by {}, which neither the file nor Nativella has: export the certificate with its chain.",
                path.display(),
                tbs.issuer
            )));
        }
        let certificates_length = std::iter::once(&certificate)
            .chain(&chain)
            .map(|certificate| certificate.to_der().map(|der| der.len()))
            .sum::<der::Result<usize>>()
            .map_err(io::Error::other)?;

        let signature_length = key.size();

        Ok(Self {
            path: path.to_owned(),
            key: SigningKey::new(key),
            signature_length,
            certificate,
            chain,
            certificates_length,
            common_name,
            team,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What Apple names the certificate: "Apple Development: <name> (<ID>)".
    pub fn common_name(&self) -> &str {
        &self.common_name
    }

    /// The team of the developer, the prefix of the application identifiers
    /// of its apps.
    pub fn team(&self) -> &str {
        &self.team
    }

    /// The certificate, in DER, as provisioning profiles list them.
    pub fn der(&self) -> io::Result<Vec<u8>> {
        self.certificate.to_der().map_err(io::Error::other)
    }

    /// An upper bound of the length of the signature of a code directory:
    /// the certificates, the signature of the key, then the structure and
    /// the attributes of the CMS signature.
    pub fn signature_length_bound(&self) -> usize {
        self.certificates_length + self.signature_length + 4096
    }

    /// The CMS signature of `code_directory`, detached, as codesign writes
    /// it: the hash of the code directory as message digest, the signing
    /// time, and Apple's attributes, which list the hashes of the code
    /// directories of the signature.
    pub fn sign(&self, code_directory: &[u8]) -> io::Result<Vec<u8>> {
        let cdhash = Sha256::digest(code_directory);
        let content = EncapsulatedContentInfo {
            econtent_type: ID_DATA,
            econtent: None,
        };
        let digest_algorithm = AlgorithmIdentifierOwned {
            oid: ID_SHA256,
            parameters: Some(Any::null()),
        };
        let signer = SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: self.certificate.tbs_certificate.issuer.clone(),
            serial_number: self.certificate.tbs_certificate.serial_number.clone(),
        });

        let mut signer_info = SignerInfoBuilder::new(
            &self.key,
            signer,
            digest_algorithm.clone(),
            &content,
            Some(&cdhash),
        )
        .map_err(cms_error)?;
        signer_info
            .add_signed_attribute(create_signing_time_attribute().map_err(cms_error)?)
            .map_err(cms_error)?;
        signer_info
            .add_signed_attribute(cdhashes(&cdhash)?)
            .map_err(cms_error)?;
        signer_info
            .add_signed_attribute(cdhashes2(&cdhash)?)
            .map_err(cms_error)?;

        let mut signed_data = SignedDataBuilder::new(&content);
        signed_data
            .add_digest_algorithm(digest_algorithm)
            .map_err(cms_error)?;
        for certificate in std::iter::once(&self.certificate).chain(&self.chain) {
            signed_data
                .add_certificate(CertificateChoices::Certificate(certificate.clone()))
                .map_err(cms_error)?;
        }
        signed_data
            .add_signer_info::<_, Signature>(signer_info)
            .map_err(cms_error)?;

        signed_data
            .build()
            .map_err(cms_error)?
            .to_der()
            .map_err(io::Error::other)
    }
}

/// The keys and the certificates of a PKCS #12 file.
struct Contents {
    /// In PKCS #8, in DER.
    keys: Vec<Vec<u8>>,
    certificates: Vec<x509_cert::Certificate>,
}

impl Contents {
    fn of(data: &[u8], password: &str) -> io::Result<Self> {
        let not_pkcs12 = || invalid("This is not a PKCS #12 file (.p12).");
        let pfx = Pfx::from_der(data).map_err(|_| not_pkcs12())?;
        if pfx.auth_safe.content_type != ID_DATA {
            return Err(invalid(
                "This PKCS #12 file is protected by a public key, not by a password.",
            ));
        }
        let safe = pfx
            .auth_safe
            .content
            .decode_as::<OctetString>()
            .map_err(|_| not_pkcs12())?;
        if let Some(mac) = &pfx.mac_data {
            verify_mac(mac, password, safe.as_bytes())?;
        }

        let mut contents = Self {
            keys: Vec::new(),
            certificates: Vec::new(),
        };
        let safe = Vec::<ContentInfo>::from_der(safe.as_bytes()).map_err(|_| not_pkcs12())?;
        for part in safe {
            let bags = match part.content_type {
                ID_DATA => part
                    .content
                    .decode_as::<OctetString>()
                    .map_err(|_| not_pkcs12())?
                    .into_bytes(),
                ID_ENCRYPTED_DATA => {
                    let encrypted = part
                        .content
                        .decode_as::<EncryptedData>()
                        .map_err(|_| not_pkcs12())?
                        .enc_content_info;
                    let data = encrypted.encrypted_content.ok_or_else(not_pkcs12)?;
                    decrypt(&encrypted.content_enc_alg, data.as_bytes(), password)?
                }
                _ => continue,
            };

            for bag in SafeContents::from_der(&bags).map_err(|_| not_pkcs12())? {
                // The value of a bag is explicitly tagged [0].
                let value = ContextSpecific::<Any>::from_der(&bag.bag_value)
                    .map_err(|_| not_pkcs12())?
                    .value
                    .to_der()
                    .map_err(io::Error::other)?;
                match bag.bag_id {
                    pkcs12::PKCS_12_KEY_BAG_OID => contents.keys.push(value),
                    pkcs12::PKCS_12_PKCS8_KEY_BAG_OID => {
                        let encrypted =
                            EncryptedPrivateKeyInfo::from_der(&value).map_err(|_| not_pkcs12())?;
                        contents.keys.push(decrypt(
                            &encrypted.encryption_algorithm,
                            encrypted.encrypted_data.as_bytes(),
                            password,
                        )?);
                    }
                    pkcs12::PKCS_12_CERT_BAG_OID => {
                        let bag = CertBag::from_der(&value).map_err(|_| not_pkcs12())?;
                        if bag.cert_id == ID_X509_CERTIFICATE {
                            contents.certificates.push(
                                x509_cert::Certificate::from_der(bag.cert_value.as_bytes())
                                    .map_err(|_| not_pkcs12())?,
                            );
                        }
                    }
                    // CRLs, secrets.
                    _ => {}
                }
            }
        }

        Ok(contents)
    }
}

/// Checks the password against the MAC of the file, which covers its
/// content: decrypting with a wrong password could succeed by chance.
fn verify_mac(mac: &MacData, password: &str, data: &[u8]) -> io::Result<()> {
    let salt = mac.mac_salt.as_bytes();
    let expected = mac.mac.digest.as_bytes();
    let wrong_password = || {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The password is wrong: set NATIVELLA_CERTIFICATE_PASSWORD to the one of the certificate.",
        )
    };

    let verified = match mac.mac.algorithm.oid {
        ID_SHA1 => {
            let key = kdf::derive_key_utf8::<Sha1>(
                password,
                salt,
                Pkcs12KeyType::Mac,
                mac.iterations,
                20,
            )
            .map_err(io::Error::other)?;
            let mut hmac = Hmac::<Sha1>::new_from_slice(&key).map_err(io::Error::other)?;
            hmac.update(data);
            hmac.verify_slice(expected).is_ok()
        }
        ID_SHA256 => {
            let key = kdf::derive_key_utf8::<Sha256>(
                password,
                salt,
                Pkcs12KeyType::Mac,
                mac.iterations,
                32,
            )
            .map_err(io::Error::other)?;
            let mut hmac = Hmac::<Sha256>::new_from_slice(&key).map_err(io::Error::other)?;
            hmac.update(data);
            hmac.verify_slice(expected).is_ok()
        }
        oid => {
            return Err(invalid(&format!(
                "This PKCS #12 file is checked with the algorithm {oid}, which this command does not know: SHA-1 or SHA-256 only."
            )));
        }
    };

    if verified {
        Ok(())
    } else {
        Err(wrong_password())
    }
}

/// Decrypts `data`, encrypted with `algorithm`: PBES2, as OpenSSL 3
/// encrypts by default, or the PBEs of PKCS #12 with 3DES or RC2, as Keychain
/// Access, and OpenSSL with `-legacy`, do.
fn decrypt(
    algorithm: &AlgorithmIdentifierOwned,
    data: &[u8],
    password: &str,
) -> io::Result<Vec<u8>> {
    let wrong_password = || {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot decrypt the file: is NATIVELLA_CERTIFICATE_PASSWORD its password?",
        )
    };

    if algorithm.oid == ID_PBES2 {
        let algorithm = algorithm.to_der().map_err(io::Error::other)?;
        let scheme = pkcs5::EncryptionScheme::try_from(algorithm.as_slice()).map_err(|_| {
            invalid("This PKCS #12 file is encrypted in a way this command does not know.")
        })?;
        return scheme
            .decrypt(password.as_bytes(), data)
            .map_err(|_| wrong_password());
    }

    let parameters = algorithm
        .parameters
        .as_ref()
        .ok_or_else(|| invalid("The encryption of this PKCS #12 file has no parameters."))?
        .decode_as::<Pkcs12PbeParams>()
        .map_err(|_| invalid("The encryption of this PKCS #12 file has invalid parameters."))?;
    let derive = |kind, length| {
        kdf::derive_key_utf8::<Sha1>(
            password,
            parameters.salt.as_bytes(),
            kind,
            parameters.iterations,
            length,
        )
        .map_err(io::Error::other)
    };
    let iv = derive(Pkcs12KeyType::Iv, 8)?;

    let decrypted = match algorithm.oid {
        pkcs12::PKCS_12_PBE_WITH_SHAAND3_KEY_TRIPLE_DES_CBC => {
            cbc::Decryptor::<des::TdesEde3>::new_from_slices(
                &derive(Pkcs12KeyType::EncryptionKey, 24)?,
                &iv,
            )
            .map_err(io::Error::other)?
            .decrypt_padded_vec_mut::<Pkcs7>(data)
        }
        pkcs12::PKCS_12_PBE_WITH_SHAAND128_BIT_RC2_CBC => {
            cbc::Decryptor::<rc2::Rc2>::new_from_slices(
                &derive(Pkcs12KeyType::EncryptionKey, 16)?,
                &iv,
            )
            .map_err(io::Error::other)?
            .decrypt_padded_vec_mut::<Pkcs7>(data)
        }
        pkcs12::PKCS_12_PBEWITH_SHAAND40_BIT_RC2_CBC => {
            cbc::Decryptor::<rc2::Rc2>::new_from_slices(
                &derive(Pkcs12KeyType::EncryptionKey, 5)?,
                &iv,
            )
            .map_err(io::Error::other)?
            .decrypt_padded_vec_mut::<Pkcs7>(data)
        }
        oid => {
            return Err(invalid(&format!(
                "This PKCS #12 file is encrypted with the algorithm {oid}, which this command does not know."
            )));
        }
    };

    decrypted.map_err(|_| wrong_password())
}

/// The issuers of `certificate`, from `candidates` or Apple's, up to a root,
/// a self-signed certificate: empty when its issuer is unknown.
fn chain(
    certificate: &x509_cert::Certificate,
    candidates: &[x509_cert::Certificate],
) -> io::Result<Vec<x509_cert::Certificate>> {
    let apple = APPLE_CERTIFICATES
        .iter()
        .map(|der| x509_cert::Certificate::from_der(der))
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;

    let mut chain: Vec<x509_cert::Certificate> = Vec::new();
    let mut current = certificate;
    // A chain cannot be longer than the certificates there are.
    for _ in 0..=candidates.len() + apple.len() {
        let tbs = &current.tbs_certificate;
        if tbs.issuer == tbs.subject {
            break;
        }
        let Some(issuer) = candidates
            .iter()
            .chain(&apple)
            .find(|candidate| candidate.tbs_certificate.subject == tbs.issuer)
        else {
            break;
        };
        chain.push(issuer.clone());
        current = chain.last().expect("An issuer was just pushed.");
    }

    Ok(chain)
}

/// The value of the attribute `oid` of `name`, as a string.
fn attribute(name: &Name, oid: ObjectIdentifier) -> Option<String> {
    name.0
        .iter()
        .flat_map(|names| names.0.iter())
        .find(|attribute| attribute.oid == oid)
        .and_then(|attribute| {
            let value = &attribute.value;
            match value.tag() {
                Tag::Utf8String | Tag::PrintableString | Tag::Ia5String => {
                    String::from_utf8(value.value().to_vec()).ok()
                }
                _ => None,
            }
        })
}

/// Apple's attribute that lists the hashes of the code directories, each
/// truncated: a property list, in an octet string.
fn cdhashes(cdhash: &[u8]) -> io::Result<Attribute> {
    let mut list = plist::Dictionary::new();
    list.insert(
        "cdhashes".to_owned(),
        vec![plist::Value::Data(cdhash[..CDHASH_LENGTH].to_vec())].into(),
    );
    let mut xml = Vec::new();
    plist::to_writer_xml(&mut xml, &list).map_err(io::Error::other)?;

    attribute_of(
        APPLE_CDHASHES,
        OctetString::new(xml).map_err(io::Error::other)?.to_der(),
    )
}

/// Apple's attribute that lists the hashes of the code directories, with
/// their algorithm: `SEQUENCE { OBJECT IDENTIFIER, OCTET STRING }`.
fn cdhashes2(cdhash: &[u8]) -> io::Result<Attribute> {
    let algorithm = ID_SHA256.to_der().map_err(io::Error::other)?;
    let hash = OctetString::new(cdhash)
        .map_err(io::Error::other)?
        .to_der()
        .map_err(io::Error::other)?;
    let mut content = algorithm;
    content.extend(hash);
    let sequence = Any::new(Tag::Sequence, content).map_err(io::Error::other)?;

    attribute_of(APPLE_CDHASHES2, sequence.to_der())
}

/// A signed attribute of the single value `value`, in DER.
fn attribute_of(oid: ObjectIdentifier, value: der::Result<Vec<u8>>) -> io::Result<Attribute> {
    let value = Any::from_der(&value.map_err(io::Error::other)?).map_err(io::Error::other)?;
    let mut values = SetOfVec::new();
    values.insert(value).map_err(io::Error::other)?;

    Ok(Attribute { oid, values })
}

fn cms_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("Cannot sign: {error}."))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
pub(crate) mod tests {
    use cms::signed_data::SignedData;
    use rsa::RsaPublicKey;
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;

    use super::*;

    /// The password of the PKCS #12 files of testdata/.
    pub const PASSWORD: &str = "nativella";

    /// A file of testdata/, made with OpenSSL 3 for a hundred years: a root
    /// and an intermediate certificate, standing for Apple's, the second with
    /// the extension of Apple's intermediate certificates of developers; a
    /// certificate of developer they issue, with the extension of Apple
    /// Development certificates, for the team TEAM123456:
    ///
    /// ```text
    /// openssl req -x509 -newkey rsa:2048 -nodes -keyout root.key -out root.pem -days 36500 \
    ///     -subj "/CN=Nativella Test Root CA/O=Nativella Tests/C=FR" \
    ///     -addext "basicConstraints=critical,CA:true" -addext "keyUsage=critical,keyCertSign,cRLSign"
    /// openssl req -newkey rsa:2048 -nodes -keyout wwdr.key -out wwdr.csr \
    ///     -subj "/CN=Nativella Test Developer Relations CA/OU=G0/O=Nativella Tests/C=FR"
    /// openssl x509 -req -in wwdr.csr -CA root.pem -CAkey root.key -set_serial 2 -days 36500 \
    ///     -out wwdr.pem -extfile wwdr.ext
    /// # wwdr.ext: basicConstraints=critical,CA:true,pathlen:0
    /// #           keyUsage=critical,keyCertSign,cRLSign
    /// #           1.2.840.113635.100.6.2.1=DER:0500
    /// openssl req -newkey rsa:2048 -nodes -keyout developer.key -out developer.csr \
    ///     -subj "/UID=TEST000001/CN=Apple Development: Nativella Tests (TEST000001)/OU=TEAM123456/O=Nativella Tests/C=FR"
    /// openssl x509 -req -in developer.csr -CA wwdr.pem -CAkey wwdr.key -set_serial 3 -days 36500 \
    ///     -out developer.pem -extfile developer.ext
    /// # developer.ext: basicConstraints=critical,CA:false
    /// #                keyUsage=critical,digitalSignature
    /// #                extendedKeyUsage=critical,codeSigning
    /// #                1.2.840.113635.100.6.1.2=critical,DER:0500
    /// ```
    ///
    /// - developer.p12: the certificate, its key and its chain, as OpenSSL 3
    ///   encrypts them (PBES2, AES-256-CBC; a MAC with SHA-256): `openssl
    ///   pkcs12 -export -inkey developer.key -in developer.pem -certfile
    ///   chain.pem -passout pass:nativella`, chain.pem holding wwdr.pem and
    ///   root.pem;
    /// - developer-legacy.p12: the certificate and its key alone, as
    ///   Keychain Access encrypts them (3DES, RC2; a MAC with SHA-1): the
    ///   same, with `-legacy`, without `-certfile`;
    /// - development.mobileprovision: a provisioning profile of the team for
    ///   `TEAM123456.dev.nativella.*`, which lists the certificate, signed by
    ///   the root: `openssl cms -sign -nodetach -binary -outform DER -in
    ///   profile.plist -signer root.pem -inkey root.key`.
    pub fn testdata(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/ios/testdata")
            .join(name)
    }

    #[test]
    fn a_certificate_of_openssl_3_with_its_chain() {
        let certificate = Certificate::load(&testdata("developer.p12"), PASSWORD).unwrap();

        assert_eq!(
            certificate.common_name(),
            "Apple Development: Nativella Tests (TEST000001)"
        );
        assert_eq!(certificate.team(), "TEAM123456");
        let chain: Vec<_> = certificate
            .chain
            .iter()
            .map(|issuer| attribute(&issuer.tbs_certificate.subject, ID_COMMON_NAME).unwrap())
            .collect();
        assert_eq!(
            chain,
            [
                "Nativella Test Developer Relations CA",
                "Nativella Test Root CA"
            ]
        );
    }

    /// Decrypted, but issued by an intermediate certificate that is not
    /// Apple's: Apple's would be found without the file.
    #[test]
    fn a_certificate_of_keychain_access_without_its_chain() {
        let error = Certificate::load(&testdata("developer-legacy.p12"), PASSWORD)
            .err()
            .unwrap()
            .to_string();

        assert!(
            error.contains("Nativella Test Developer Relations CA"),
            "{error}"
        );
        assert!(error.contains("with its chain"), "{error}");
    }

    #[test]
    fn a_wrong_password() {
        for name in ["developer.p12", "developer-legacy.p12"] {
            let error = Certificate::load(&testdata(name), "wrong").err().unwrap();

            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied, "{name}");
        }
    }

    #[test]
    fn apple_certificates_chain_to_their_roots() {
        let apple = APPLE_CERTIFICATES
            .iter()
            .map(|der| x509_cert::Certificate::from_der(der).unwrap())
            .collect::<Vec<_>>();

        // The intermediate certificates, then the roots.
        for intermediate in &apple[..4] {
            let chain = chain(intermediate, &[]).unwrap();
            assert_eq!(chain.len(), 1);
            let root = &chain[0].tbs_certificate;
            assert_eq!(root.subject, root.issuer);
        }
    }

    #[test]
    fn the_signature_of_a_code_directory() {
        let certificate = Certificate::load(&testdata("developer.p12"), PASSWORD).unwrap();
        let code_directory = b"a code directory";

        let signature = certificate.sign(code_directory).unwrap();

        assert!(signature.len() <= certificate.signature_length_bound());
        let signed = ContentInfo::from_der(&signature)
            .unwrap()
            .content
            .decode_as::<SignedData>()
            .unwrap();
        assert!(signed.encap_content_info.econtent.is_none(), "Detached.");
        assert_eq!(signed.certificates.unwrap().0.len(), 3);
        let signer = signed.signer_infos.0.iter().next().unwrap();
        let attributes = signer.signed_attrs.as_ref().unwrap();
        let value = |oid: &str| {
            let oid = ObjectIdentifier::new_unwrap(oid);
            attributes
                .iter()
                .find(|attribute| attribute.oid == oid)
                .unwrap()
                .values
                .iter()
                .next()
                .unwrap()
                .clone()
        };
        let cdhash = Sha256::digest(code_directory);

        // The message digest, then Apple's hashes.
        let digest = value("1.2.840.113549.1.9.4")
            .decode_as::<OctetString>()
            .unwrap();
        assert_eq!(digest.as_bytes(), cdhash.as_slice());
        let hashes = value("1.2.840.113635.100.9.2");
        assert_eq!(hashes.tag(), Tag::Sequence);
        assert!(hashes.value().ends_with(&cdhash));
        let list = value("1.2.840.113635.100.9.1")
            .decode_as::<OctetString>()
            .unwrap();
        let list: plist::Dictionary = plist::from_bytes(list.as_bytes()).unwrap();
        assert_eq!(
            list["cdhashes"].as_array().unwrap()[0].as_data().unwrap(),
            &cdhash[..CDHASH_LENGTH]
        );

        // Signed by the key of the certificate.
        let public = RsaPublicKey::from_public_key_der(
            &certificate
                .certificate
                .tbs_certificate
                .subject_public_key_info
                .to_der()
                .unwrap(),
        )
        .unwrap();
        VerifyingKey::<Sha256>::new(public)
            .verify(
                &attributes.to_der().unwrap(),
                &Signature::try_from(signer.signature.as_bytes()).unwrap(),
            )
            .unwrap();
    }
}
