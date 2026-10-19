//! The code signature of an iOS app, as Apple's codesign writes it.
//!
//! The executable carries it, at its end (see macho.rs): a superblob of
//! blobs, each in a slot.
//!
//! - The code directory (slot 0) holds the SHA-256 hash of each page of 4 KiB
//!   of the executable before the signature, and of the other blobs and files
//!   it binds, the "special slots": the Info.plist (1), the requirements (2),
//!   `_CodeSignature/CodeResources` (3), which seals the other files of the
//!   app (see resources.rs), and the entitlements (5), as XML, and as DER (7).
//! - The requirements (2): the designated requirement, which another build of
//!   the same app, signed by the same developer, meets.
//! - The entitlements (5 and 7): what the app may do, from the provisioning
//!   profile (see entitlements.rs).
//! - The CMS signature (0x10000) of the code directory, by the certificate of
//!   the developer (see certificate.rs); empty for an ad hoc signature, which
//!   only hashes.
//!
//! The formats are those of Apple's open source: `cs_blobs.h` of xnu for the
//! blobs, `requirement.h` of Security for the requirements. They were checked
//! against an app signed by Xcode, byte for byte for the requirements and the
//! entitlements; the Mobile workflow checks ad hoc signatures with `codesign
//! --verify`.

use std::fs;
use std::io;
use std::path::Path;

use plist::{Dictionary, Value};
use rsa::sha2::{Digest, Sha256};

use super::certificate::Certificate;
use super::entitlements;
use super::macho::{Executable, PAGE_SIZE};
use super::resources;

const SUPERBLOB: u32 = 0xfade_0cc0;
const CODE_DIRECTORY: u32 = 0xfade_0c02;
const REQUIREMENTS: u32 = 0xfade_0c01;
const REQUIREMENT: u32 = 0xfade_0c00;
const ENTITLEMENTS: u32 = 0xfade_7171;
const DER_ENTITLEMENTS: u32 = 0xfade_7172;
/// The CMS signature, in a blob wrapper.
const BLOB_WRAPPER: u32 = 0xfade_0b01;

const SLOT_CODE_DIRECTORY: u32 = 0;
const SLOT_INFO: u32 = 1;
const SLOT_REQUIREMENTS: u32 = 2;
const SLOT_RESOURCES: u32 = 3;
const SLOT_ENTITLEMENTS: u32 = 5;
const SLOT_DER_ENTITLEMENTS: u32 = 7;
const SLOT_SIGNATURE: u32 = 0x10000;

/// With the scatter, team, 64-bit code limit and executable segment fields.
const CODE_DIRECTORY_VERSION: u32 = 0x20400;
const CODE_DIRECTORY_HEADER_LENGTH: usize = 88;
const HASH_SHA256: u8 = 2;
const HASH_LENGTH: usize = 32;
const CS_ADHOC: u32 = 0x2;
const CS_EXECSEG_MAIN_BINARY: u64 = 0x1;
/// Debuggers may attach: an app with the entitlement `get-task-allow`, as
/// development profiles grant.
const CS_EXECSEG_ALLOW_UNSIGNED: u64 = 0x10;

/// The type of the designated requirement, in a requirement set.
const DESIGNATED_REQUIREMENT: u32 = 3;
/// A requirement written as an expression.
const EXPRESSION: u32 = 1;
const OP_IDENT: u32 = 2;
const OP_AND: u32 = 6;
const OP_CERT_FIELD: u32 = 11;
const OP_CERT_GENERIC: u32 = 14;
const OP_APPLE_GENERIC_ANCHOR: u32 = 15;
const MATCH_EXISTS: u32 = 0;
const MATCH_EQUAL: u32 = 1;
/// The certificate of the developer, in a requirement; its issuer is 1.
const LEAF: u32 = 0;
/// 1.2.840.113635.100.6.2.1, in DER: the extension of Apple's intermediate
/// certificates of developers, the Worldwide Developer Relations ones.
const WWDR_EXTENSION: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x63, 0x64, 0x06, 0x02, 0x01];

/// Who signs.
pub enum Signer<'a> {
    /// Nobody: the signature only hashes, which the simulator accepts.
    AdHoc,
    /// The developer, with the entitlements of the provisioning profile.
    Developer {
        certificate: &'a Certificate,
        entitlements: &'a Dictionary,
    },
}

/// Signs the app `app`: seals its files in `_CodeSignature/CodeResources`,
/// then signs its executable, which binds them.
pub fn sign(app: &Path, signer: &Signer) -> io::Result<()> {
    let info_bytes = fs::read(app.join("Info.plist"))?;
    let info: Dictionary = plist::from_bytes(&info_bytes)
        .map_err(|error| invalid(&format!("The Info.plist of the app is invalid: {error}.")))?;
    let field = |key: &str| {
        info.get(key)
            .and_then(Value::as_string)
            .ok_or_else(|| invalid(&format!("The Info.plist of the app has no {key}.")))
    };
    let executable = field("CFBundleExecutable")?;
    let identifier = field("CFBundleIdentifier")?;

    let code_resources = resources::seal(app, executable)?;
    let signature_dir = app.join("_CodeSignature");
    fs::create_dir_all(&signature_dir)?;
    fs::write(signature_dir.join("CodeResources"), &code_resources)?;

    // By slot, from 1.
    let mut special = [None; 7];
    special[SLOT_INFO as usize - 1] = Some(sha256(&info_bytes));
    special[SLOT_RESOURCES as usize - 1] = Some(sha256(&code_resources));

    let mut blobs = Vec::new();
    let (flags, team) = match signer {
        Signer::AdHoc => {
            blobs.push((SLOT_REQUIREMENTS, requirements(None)));
            (CS_ADHOC, None)
        }
        Signer::Developer {
            certificate,
            entitlements,
        } => {
            blobs.push((
                SLOT_REQUIREMENTS,
                requirements(Some(&designated_requirement(
                    identifier,
                    certificate.common_name(),
                ))),
            ));
            blobs.push((
                SLOT_ENTITLEMENTS,
                blob(ENTITLEMENTS, &entitlements::xml(entitlements)?),
            ));
            blobs.push((
                SLOT_DER_ENTITLEMENTS,
                blob(DER_ENTITLEMENTS, &entitlements::der(entitlements)?),
            ));
            (0, Some(certificate.team()))
        }
    };
    for (slot, data) in &blobs {
        special[*slot as usize - 1] = Some(sha256(data));
    }
    let exec_segment_flags = match signer {
        Signer::Developer { entitlements, .. }
            if entitlements
                .get("get-task-allow")
                .and_then(Value::as_boolean)
                == Some(true) =>
        {
            CS_EXECSEG_MAIN_BINARY | CS_EXECSEG_ALLOW_UNSIGNED
        }
        _ => CS_EXECSEG_MAIN_BINARY,
    };

    let mut executable = Executable::open(&app.join(executable))?;
    let code_slots = executable.signature_offset.div_ceil(PAGE_SIZE) as usize;
    let special_slots = special
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |last| last + 1);
    let code_directory_length = CODE_DIRECTORY_HEADER_LENGTH
        + identifier.len()
        + 1
        + team.map_or(0, |team| team.len() + 1)
        + (special_slots + code_slots) * HASH_LENGTH;
    // The blob wrapper, empty for an ad hoc signature.
    let signature_length = 8 + match signer {
        Signer::AdHoc => 0,
        Signer::Developer { certificate, .. } => certificate.signature_length_bound(),
    };
    // The superblob, its index, then its blobs.
    let reserved = 12
        + 8 * (blobs.len() + 2)
        + code_directory_length
        + blobs.iter().map(|(_, data)| data.len()).sum::<usize>()
        + signature_length;
    let reserved = u32::try_from(reserved.next_multiple_of(16))
        .map_err(|_| invalid("The signature would exceed 4 GB."))?;

    executable.reserve(reserved)?;
    let code_directory = code_directory(&CodeDirectory {
        identifier,
        team,
        flags,
        code_limit: executable.signature_offset,
        special: &special[..special_slots],
        code: &executable.page_hashes()?,
        exec_segment: executable.text,
        exec_segment_flags,
    })?;
    let signature = match signer {
        Signer::AdHoc => Vec::new(),
        Signer::Developer { certificate, .. } => certificate.sign(&code_directory)?,
    };

    blobs.insert(0, (SLOT_CODE_DIRECTORY, code_directory));
    blobs.push((SLOT_SIGNATURE, blob(BLOB_WRAPPER, &signature)));
    executable.write_signature(&superblob(&blobs), reserved)
}

/// What the code directory holds.
struct CodeDirectory<'a> {
    identifier: &'a str,
    team: Option<&'a str>,
    flags: u32,
    /// Where the signed code ends, and the signature starts.
    code_limit: u64,
    /// The hashes of the special slots, from slot 1.
    special: &'a [Option<[u8; HASH_LENGTH]>],
    /// The hashes of the pages of the code.
    code: &'a [[u8; HASH_LENGTH]],
    /// Where `__TEXT` starts and ends.
    exec_segment: (u64, u64),
    exec_segment_flags: u64,
}

/// The code directory: its header, the identifier and the team, then the
/// hashes, those of the special slots before the hash offset, in reverse
/// order.
fn code_directory(directory: &CodeDirectory) -> io::Result<Vec<u8>> {
    let identifier_offset = CODE_DIRECTORY_HEADER_LENGTH;
    let team_offset = identifier_offset + directory.identifier.len() + 1;
    let hash_offset = team_offset
        + directory.team.map_or(0, |team| team.len() + 1)
        + directory.special.len() * HASH_LENGTH;
    let length = hash_offset + directory.code.len() * HASH_LENGTH;
    let code_limit = u32::try_from(directory.code_limit)
        .map_err(|_| invalid("The executable of the template exceeds 4 GB."))?;
    let u32_of = |value: usize| {
        u32::try_from(value).map_err(|_| invalid("The code directory would exceed 4 GB."))
    };

    let mut data = Vec::with_capacity(length);
    data.extend(CODE_DIRECTORY.to_be_bytes());
    data.extend(u32_of(length)?.to_be_bytes());
    data.extend(CODE_DIRECTORY_VERSION.to_be_bytes());
    data.extend(directory.flags.to_be_bytes());
    data.extend(u32_of(hash_offset)?.to_be_bytes());
    data.extend(u32_of(identifier_offset)?.to_be_bytes());
    data.extend(u32_of(directory.special.len())?.to_be_bytes());
    data.extend(u32_of(directory.code.len())?.to_be_bytes());
    data.extend(code_limit.to_be_bytes());
    // The size and the type of the hashes, the platform, the size of pages,
    // as a power of 2.
    data.extend([HASH_LENGTH as u8, HASH_SHA256, 0, PAGE_SIZE.ilog2() as u8]);
    // Spare, then no scatter.
    data.extend([0; 8]);
    let team_offset = if directory.team.is_some() {
        u32_of(team_offset)?
    } else {
        0
    };
    data.extend(team_offset.to_be_bytes());
    // Spare, then the 64-bit code limit, which a code limit under 4 GB
    // leaves at 0.
    data.extend([0; 12]);
    data.extend(directory.exec_segment.0.to_be_bytes());
    data.extend(directory.exec_segment.1.to_be_bytes());
    data.extend(directory.exec_segment_flags.to_be_bytes());
    debug_assert_eq!(data.len(), CODE_DIRECTORY_HEADER_LENGTH);

    data.extend(directory.identifier.as_bytes());
    data.push(0);
    if let Some(team) = directory.team {
        data.extend(team.as_bytes());
        data.push(0);
    }
    for hash in directory.special.iter().rev() {
        data.extend(hash.unwrap_or_default());
    }
    for hash in directory.code {
        data.extend(hash);
    }

    Ok(data)
}

/// The requirement set: the designated requirement alone, or none for an ad
/// hoc signature.
fn requirements(designated: Option<&[u8]>) -> Vec<u8> {
    let Some(designated) = designated else {
        return blob(REQUIREMENTS, &0_u32.to_be_bytes());
    };

    let mut data = Vec::new();
    // One requirement, its type and its offset in the blob: after the magic,
    // the length, the count, the type and the offset.
    data.extend(1_u32.to_be_bytes());
    data.extend(DESIGNATED_REQUIREMENT.to_be_bytes());
    data.extend(20_u32.to_be_bytes());
    data.extend(designated);

    blob(REQUIREMENTS, &data)
}

/// What codesign requires of another build of the app `identifier`, signed
/// by the developer certificate `common_name`, which Apple issued:
///
/// ```text
/// identifier "<identifier>" and anchor apple generic
///     and certificate leaf[subject.CN] = "<common name>"
///     and certificate 1[field.1.2.840.113635.100.6.2.1] /* exists */
/// ```
fn designated_requirement(identifier: &str, common_name: &str) -> Vec<u8> {
    let expression = Expression(EXPRESSION.to_be_bytes().to_vec())
        .op(OP_AND)
        .op(OP_IDENT)
        .data(identifier.as_bytes())
        .op(OP_AND)
        .op(OP_APPLE_GENERIC_ANCHOR)
        .op(OP_AND)
        .op(OP_CERT_FIELD)
        .op(LEAF)
        .data(b"subject.CN")
        .op(MATCH_EQUAL)
        .data(common_name.as_bytes())
        .op(OP_CERT_GENERIC)
        .op(LEAF + 1)
        .data(WWDR_EXTENSION)
        .op(MATCH_EXISTS);

    blob(REQUIREMENT, &expression.0)
}

/// A requirement, in its binary form: operators and their operands, in
/// prefix order.
struct Expression(Vec<u8>);

impl Expression {
    /// An operator, or an integer operand.
    fn op(mut self, value: u32) -> Self {
        self.0.extend(value.to_be_bytes());
        self
    }

    /// A string or binary operand: its length, then its bytes, padded to 4
    /// bytes.
    fn data(mut self, data: &[u8]) -> Self {
        self.0.extend((data.len() as u32).to_be_bytes());
        self.0.extend(data);
        self.0.resize(self.0.len().next_multiple_of(4), 0);
        self
    }
}

/// A blob: its magic, its length, then its data.
fn blob(magic: u32, data: &[u8]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(8 + data.len());
    blob.extend(magic.to_be_bytes());
    blob.extend((8 + data.len() as u32).to_be_bytes());
    blob.extend(data);

    blob
}

/// The superblob of `blobs`, in the order of their slots: its index, then
/// the blobs.
fn superblob(blobs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let index_length = 12 + 8 * blobs.len();
    let length = index_length + blobs.iter().map(|(_, data)| data.len()).sum::<usize>();

    let mut superblob = Vec::with_capacity(length);
    superblob.extend(SUPERBLOB.to_be_bytes());
    superblob.extend((length as u32).to_be_bytes());
    superblob.extend((blobs.len() as u32).to_be_bytes());
    let mut offset = index_length;
    for (slot, data) in blobs {
        superblob.extend(slot.to_be_bytes());
        superblob.extend((offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (_, data) in blobs {
        superblob.extend(data);
    }

    superblob
}

fn sha256(data: &[u8]) -> [u8; HASH_LENGTH] {
    Sha256::digest(data).into()
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    /// The blobs of the signature of the executable `path`, by slot.
    pub fn blobs(path: &Path) -> BTreeMap<u32, Vec<u8>> {
        let offset = Executable::open(path).unwrap().signature_offset as usize;
        let bytes = fs::read(path).unwrap();
        let superblob = &bytes[offset..];
        assert_eq!(u32_at(superblob, 0), SUPERBLOB);

        (0..u32_at(superblob, 8) as usize)
            .map(|index| {
                let slot = u32_at(superblob, 12 + 8 * index);
                let start = u32_at(superblob, 16 + 8 * index) as usize;
                let length = u32_at(superblob, start + 4) as usize;
                (slot, superblob[start..start + length].to_vec())
            })
            .collect()
    }

    /// What a code directory holds: its flags, identifier, team, the hashes
    /// of its special slots, from 1, and of the code.
    pub struct Parsed {
        pub flags: u32,
        pub identifier: String,
        pub team: Option<String>,
        pub special: Vec<[u8; HASH_LENGTH]>,
        pub code: Vec<[u8; HASH_LENGTH]>,
        pub code_limit: u32,
        pub exec_segment_flags: u64,
    }

    pub fn parse(directory: &[u8]) -> Parsed {
        assert_eq!(u32_at(directory, 0), CODE_DIRECTORY);
        assert_eq!(u32_at(directory, 4) as usize, directory.len());
        assert_eq!(u32_at(directory, 8), CODE_DIRECTORY_VERSION);
        let string = |offset: usize| {
            let end = offset
                + directory[offset..]
                    .iter()
                    .position(|&byte| byte == 0)
                    .unwrap();
            String::from_utf8(directory[offset..end].to_vec()).unwrap()
        };
        let hash_offset = u32_at(directory, 16) as usize;
        let special = u32_at(directory, 24) as usize;
        let code = u32_at(directory, 28) as usize;
        let hash = |offset: usize| -> [u8; HASH_LENGTH] {
            directory[offset..offset + HASH_LENGTH].try_into().unwrap()
        };
        assert_eq!(directory[36..40], [32, HASH_SHA256, 0, 12]);
        let team = u32_at(directory, 48) as usize;

        Parsed {
            flags: u32_at(directory, 12),
            identifier: string(u32_at(directory, 20) as usize),
            team: (team != 0).then(|| string(team)),
            special: (1..=special)
                .map(|slot| hash(hash_offset - slot * HASH_LENGTH))
                .collect(),
            code: (0..code)
                .map(|page| hash(hash_offset + page * HASH_LENGTH))
                .collect(),
            code_limit: u32_at(directory, 32),
            exec_segment_flags: u64::from_be_bytes(directory[80..88].try_into().unwrap()),
        }
    }

    #[test]
    fn a_code_directory_as_codesign_lays_it_out() {
        let directory = code_directory(&CodeDirectory {
            identifier: "com.acme.notes",
            team: Some("TEAM123456"),
            flags: 0,
            code_limit: 0x1234,
            special: &[Some([1; 32]), None, Some([3; 32])],
            code: &[[0xa; 32], [0xb; 32]],
            exec_segment: (0, 0x1000),
            exec_segment_flags: CS_EXECSEG_MAIN_BINARY,
        })
        .unwrap();

        // The header, the identifier and the team, then the hashes.
        assert_eq!(u32_at(&directory, 20), 88);
        assert_eq!(u32_at(&directory, 48), 88 + 15);
        assert_eq!(u32_at(&directory, 16), 88 + 15 + 11 + 3 * 32);
        let parsed = parse(&directory);
        assert_eq!(parsed.identifier, "com.acme.notes");
        assert_eq!(parsed.team.as_deref(), Some("TEAM123456"));
        assert_eq!(parsed.special, [[1; 32], [0; 32], [3; 32]]);
        assert_eq!(parsed.code, [[0xa; 32], [0xb; 32]]);
        assert_eq!(parsed.code_limit, 0x1234);
        assert_eq!(directory[72..80], 0x1000_u64.to_be_bytes());
    }

    /// As an app signed by Xcode holds it, but the identifier and the common
    /// name.
    #[test]
    fn the_designated_requirement_of_a_developer() {
        let set = requirements(Some(&designated_requirement(
            "com.acme.notes",
            "Apple Development: Jane Doe (ABCDE12345)",
        )));

        let mut expected = Vec::new();
        for word in [
            REQUIREMENTS,
            172,
            1,
            DESIGNATED_REQUIREMENT,
            20, // The set.
            REQUIREMENT,
            152,
            EXPRESSION, // The requirement.
            OP_AND,
            OP_IDENT,
            14,
        ] {
            expected.extend(word.to_be_bytes());
        }
        expected.extend(b"com.acme.notes\0\0");
        for word in [
            OP_AND,
            OP_APPLE_GENERIC_ANCHOR,
            OP_AND,
            OP_CERT_FIELD,
            LEAF,
            10,
        ] {
            expected.extend(word.to_be_bytes());
        }
        expected.extend(b"subject.CN\0\0");
        for word in [MATCH_EQUAL, 40] {
            expected.extend(word.to_be_bytes());
        }
        expected.extend(b"Apple Development: Jane Doe (ABCDE12345)");
        for word in [OP_CERT_GENERIC, 1, 10] {
            expected.extend(word.to_be_bytes());
        }
        expected.extend(WWDR_EXTENSION);
        expected.extend([0, 0]);
        expected.extend(MATCH_EXISTS.to_be_bytes());
        assert_eq!(set, expected);

        assert_eq!(
            requirements(None),
            [0xfa, 0xde, 0x0c, 0x01, 0, 0, 0, 12, 0, 0, 0, 0],
            "Ad hoc signatures have no requirement."
        );
    }
}
