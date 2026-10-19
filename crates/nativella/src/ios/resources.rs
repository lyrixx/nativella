//! `_CodeSignature/CodeResources`: the seal of the files of an app, but its
//! executable, which carries the signature, and the signature directory. The
//! code directory binds it by its hash (see signature.rs).
//!
//! A property list of the SHA-1 hash of each file (`files`, the first
//! version of the seal), of its SHA-256 hash (`files2`, the second), and of
//! the rules that chose them: those codesign applies to iOS apps, as an app
//! signed by Xcode has them. The second version leaves out the Info.plist and
//! PkgInfo, which the code directory binds already; both make the localized
//! resources (`*.lproj/`) optional, but the base ones.
//!
//! Nested code, frameworks or app extensions, needs a seal of its own, signed
//! first: an app of the template has none, and none is sealed.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use plist::{Dictionary, Value};
use rsa::sha2::{Digest, Sha256};
use sha1::Sha1;

/// The magic numbers of Mach-O files, thin or universal, in either order.
const MACHO_MAGICS: [[u8; 4]; 6] = [
    [0xcf, 0xfa, 0xed, 0xfe],
    [0xce, 0xfa, 0xed, 0xfe],
    [0xfe, 0xed, 0xfa, 0xcf],
    [0xfe, 0xed, 0xfa, 0xce],
    [0xca, 0xfe, 0xba, 0xbe],
    [0xbe, 0xba, 0xfe, 0xca],
];

/// What a rule makes of a file.
#[derive(Debug, PartialEq)]
enum Rule {
    Sealed,
    /// Sealed, and may be missing.
    Optional,
    Omitted,
}

/// The hashes of a file.
struct Hashes {
    sha1: Vec<u8>,
    sha256: Vec<u8>,
}

/// The seal of the files of `app`, but its `executable`: what to write to
/// `_CodeSignature/CodeResources`.
pub fn seal(app: &Path, executable: &str) -> io::Result<Vec<u8>> {
    let mut hashes = BTreeMap::new();
    hash_dir(app, "", executable, &mut hashes)?;

    let mut files = Dictionary::new();
    let mut files2 = Dictionary::new();
    for (path, hashes) in &hashes {
        match rule(path, false) {
            Rule::Sealed => {
                files.insert(path.clone(), Value::Data(hashes.sha1.clone()));
            }
            Rule::Optional => {
                files.insert(
                    path.clone(),
                    dictionary([
                        ("hash", Value::Data(hashes.sha1.clone())),
                        ("optional", true.into()),
                    ]),
                );
            }
            Rule::Omitted => {}
        }
        match rule(path, true) {
            Rule::Sealed => {
                files2.insert(
                    path.clone(),
                    dictionary([("hash2", Value::Data(hashes.sha256.clone()))]),
                );
            }
            Rule::Optional => {
                files2.insert(
                    path.clone(),
                    dictionary([
                        ("hash2", Value::Data(hashes.sha256.clone())),
                        ("optional", true.into()),
                    ]),
                );
            }
            Rule::Omitted => {}
        }
    }

    let seal = dictionary([
        ("files", files.into()),
        ("files2", files2.into()),
        ("rules", rules()),
        ("rules2", rules2()),
    ]);
    let mut xml = Vec::new();
    plist::to_writer_xml(&mut xml, &seal)
        .map_err(|error| io::Error::other(format!("Cannot write the seal: {error}.")))?;

    Ok(xml)
}

/// Hashes the files of the directory `relative` of `app`, but the
/// `executable` and the signature directory, at the root.
fn hash_dir(
    app: &Path,
    relative: &str,
    executable: &str,
    hashes: &mut BTreeMap<String, Hashes>,
) -> io::Result<()> {
    for entry in fs::read_dir(app.join(relative))? {
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|name| {
            invalid(&format!(
                "The app holds {}, whose name is not UTF-8.",
                name.to_string_lossy()
            ))
        })?;
        let path = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        if path == executable || path == "_CodeSignature" {
            continue;
        }

        let kind = entry.file_type()?;
        if kind.is_dir() {
            hash_dir(app, &path, executable, hashes)?;
        } else if kind.is_file() {
            hashes.insert(path.clone(), hash_file(&entry.path(), &path)?);
        } else {
            return Err(invalid(&format!(
                "The app holds {path}, a symbolic link or a special file, which no app of the template has."
            )));
        }
    }

    Ok(())
}

/// The hashes of the file `path`, `relative` in the app, which must not be
/// code.
fn hash_file(path: &Path, relative: &str) -> io::Result<Hashes> {
    let mut file = File::open(path)?;
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let mut first = true;

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if first && read >= 4 && MACHO_MAGICS.iter().any(|magic| buffer.starts_with(magic)) {
            return Err(invalid(&format!(
                "The app holds {relative}, nested code, which this command does not sign."
            )));
        }
        first = false;
        sha1.update(&buffer[..read]);
        sha256.update(&buffer[..read]);
    }

    Ok(Hashes {
        sha1: sha1.finalize().to_vec(),
        sha256: sha256.finalize().to_vec(),
    })
}

/// What the first or the second version of the seal makes of the file
/// `path`: the rule of the greatest weight among those of [`rules`] or
/// [`rules2`] that match it, from the greatest weight down.
fn rule(path: &str, version2: bool) -> Rule {
    let name = path.rsplit('/').next().unwrap_or(path);

    if (version2 && name == ".DS_Store") || path.ends_with(".lproj/locversion.plist") {
        Rule::Omitted
    } else if path.starts_with("Base.lproj/") {
        Rule::Sealed
    } else if path.contains(".lproj/") {
        Rule::Optional
    } else if version2 && (path == "Info.plist" || path == "PkgInfo") {
        Rule::Omitted
    } else {
        Rule::Sealed
    }
}

/// The rules of the first version, as codesign writes them for iOS apps.
fn rules() -> Value {
    dictionary([
        ("^.*", true.into()),
        ("^.*\\.lproj/", weighted(1000.0, Some("optional"))),
        (
            "^.*\\.lproj/locversion.plist$",
            weighted(1100.0, Some("omit")),
        ),
        ("^Base\\.lproj/", weighted(1010.0, None)),
        ("^version.plist$", true.into()),
    ])
}

/// The rules of the second version, as codesign writes them for iOS apps.
fn rules2() -> Value {
    dictionary([
        (".*\\.dSYM($|/)", weighted(11.0, None)),
        ("^(.*/)?\\.DS_Store$", weighted(2000.0, Some("omit"))),
        ("^.*", true.into()),
        ("^.*\\.lproj/", weighted(1000.0, Some("optional"))),
        (
            "^.*\\.lproj/locversion.plist$",
            weighted(1100.0, Some("omit")),
        ),
        ("^Base\\.lproj/", weighted(1010.0, None)),
        ("^Info\\.plist$", weighted(20.0, Some("omit"))),
        ("^PkgInfo$", weighted(20.0, Some("omit"))),
        ("^embedded\\.provisionprofile$", weighted(20.0, None)),
        ("^version\\.plist$", weighted(20.0, None)),
    ])
}

/// A rule of the given weight, with a flag, `omit` or `optional`.
fn weighted(weight: f64, flag: Option<&str>) -> Value {
    let mut rule = Dictionary::new();
    if let Some(flag) = flag {
        rule.insert(flag.to_owned(), true.into());
    }
    rule.insert("weight".to_owned(), weight.into());

    rule.into()
}

/// A dictionary of `entries`, in their order.
fn dictionary<const N: usize>(entries: [(&str, Value); N]) -> Value {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect::<Dictionary>()
        .into()
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_as_codesign_applies_them() {
        for (path, version1, version2) in [
            ("Assets.car", Rule::Sealed, Rule::Sealed),
            ("nativella.bundle", Rule::Sealed, Rule::Sealed),
            ("Info.plist", Rule::Sealed, Rule::Omitted),
            ("PkgInfo", Rule::Sealed, Rule::Omitted),
            (
                "LaunchScreen.storyboardc/Info.plist",
                Rule::Sealed,
                Rule::Sealed,
            ),
            (
                "fr.lproj/Localizable.strings",
                Rule::Optional,
                Rule::Optional,
            ),
            ("Base.lproj/Main.storyboardc", Rule::Sealed, Rule::Sealed),
            ("fr.lproj/locversion.plist", Rule::Omitted, Rule::Omitted),
            (".DS_Store", Rule::Sealed, Rule::Omitted),
        ] {
            assert_eq!(rule(path, false), version1, "{path}");
            assert_eq!(rule(path, true), version2, "{path}");
        }
    }

    #[test]
    fn the_files_but_the_executable_are_sealed() {
        let app =
            Path::new(env!("OUT_DIR")).join(format!("resources-test-{}.app", std::process::id()));
        fs::create_dir_all(app.join("_CodeSignature")).unwrap();
        fs::create_dir_all(app.join("fr.lproj")).unwrap();
        fs::write(app.join("Nativella"), [0xcf, 0xfa, 0xed, 0xfe, 0]).unwrap();
        fs::write(app.join("_CodeSignature/CodeResources"), "previous").unwrap();
        fs::write(app.join("Info.plist"), "info").unwrap();
        fs::write(app.join("nativella.bundle"), "bundle").unwrap();
        fs::write(app.join("fr.lproj/InfoPlist.strings"), "fr").unwrap();

        let sealed: Dictionary = plist::from_bytes(&seal(&app, "Nativella").unwrap()).unwrap();

        let files = sealed["files"].as_dictionary().unwrap();
        let keys: Vec<_> = files.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "Info.plist",
                "fr.lproj/InfoPlist.strings",
                "nativella.bundle"
            ]
        );
        assert_eq!(
            files["nativella.bundle"].as_data().unwrap(),
            Sha1::digest(b"bundle").as_slice()
        );
        let files2 = sealed["files2"].as_dictionary().unwrap();
        let keys: Vec<_> = files2.keys().map(String::as_str).collect();
        assert_eq!(keys, ["fr.lproj/InfoPlist.strings", "nativella.bundle"]);
        let bundle = files2["nativella.bundle"].as_dictionary().unwrap();
        assert_eq!(
            bundle["hash2"].as_data().unwrap(),
            Sha256::digest(b"bundle").as_slice()
        );
        let strings = files2["fr.lproj/InfoPlist.strings"]
            .as_dictionary()
            .unwrap();
        assert_eq!(strings["optional"].as_boolean(), Some(true));
        assert_eq!(sealed["rules2"].as_dictionary().unwrap().len(), 10);

        // Nested code is not sealed.
        fs::write(app.join("Helper"), [0xcf, 0xfa, 0xed, 0xfe, 0]).unwrap();
        let error = seal(&app, "Nativella").unwrap_err();
        assert!(error.to_string().contains("nested code"), "{error}");

        fs::remove_dir_all(app).unwrap();
    }
}
