//! `_CodeSignature/CodeResources`: the seal of the files of an app, but its
//! executable, which carries the signature, and the signature directory. The
//! code directory binds it by its hash (see signature.rs).
//!
//! A property list of the SHA-1 hash of each file (`files`, the first
//! version of the seal), of its SHA-256 hash (`files2`, the second), and of
//! the rules that chose them: those codesign applies to the apps of each
//! [`Layout`], as an app signed by Xcode has them for iOS, and an app signed
//! ad hoc by codesign for macOS. The second version leaves out the Info.plist
//! and PkgInfo, which the code directory binds already; both make the
//! localized resources (`*.lproj/`) optional, but the base ones. On macOS,
//! the first version seals the resources only, and the second takes the
//! files of `MacOS/`, and those beside the Info.plist, for nested code.
//!
//! Nested code, frameworks or app extensions, needs a seal of its own, signed
//! first: the apps Nativella makes have none, and none is sealed.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use plist::{Dictionary, Value};
use rsa::sha2::{Digest, Sha256};
use sha1::Sha1;

use super::Layout;

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
    /// Code, signed on its own.
    Nested,
}

/// The hashes of a file.
struct Hashes {
    sha1: Vec<u8>,
    sha256: Vec<u8>,
}

/// The seal of the files of `contents`, the directory of an app laid out as
/// `layout` that holds its signature, but its `executable`, a path in it with
/// `/`: what to write to `_CodeSignature/CodeResources`.
pub fn seal(contents: &Path, executable: &str, layout: Layout) -> io::Result<Vec<u8>> {
    let mut hashes = BTreeMap::new();
    hash_dir(contents, "", executable, &mut hashes)?;

    let mut files = Dictionary::new();
    let mut files2 = Dictionary::new();
    for (path, hashes) in &hashes {
        let rules = [rule(path, false, layout), rule(path, true, layout)];
        if rules.contains(&Rule::Nested) {
            return Err(invalid(&format!(
                "The app holds {path}, where only code goes, which this command does not sign."
            )));
        }
        match rules[0] {
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
            Rule::Omitted | Rule::Nested => {}
        }
        match rules[1] {
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
            Rule::Omitted | Rule::Nested => {}
        }
    }

    let seal = dictionary([
        ("files", files.into()),
        ("files2", files2.into()),
        ("rules", rules(layout)),
        ("rules2", rules2(layout)),
    ]);
    let mut xml = Vec::new();
    plist::to_writer_xml(&mut xml, &seal)
        .map_err(|error| io::Error::other(format!("Cannot write the seal: {error}.")))?;

    Ok(xml)
}

/// Hashes the files of the directory `relative` of `contents`, but the
/// `executable` and the signature directory, at the root.
fn hash_dir(
    contents: &Path,
    relative: &str,
    executable: &str,
    hashes: &mut BTreeMap<String, Hashes>,
) -> io::Result<()> {
    for entry in fs::read_dir(contents.join(relative))? {
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
            hash_dir(contents, &path, executable, hashes)?;
        } else if kind.is_file() {
            hashes.insert(path.clone(), hash_file(&entry.path(), &path)?);
        } else {
            return Err(invalid(&format!(
                "The app holds {path}, a symbolic link or a special file, which no app of Nativella holds."
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

/// What the first or the second version of the seal of an app laid out as
/// `layout` makes of the file `path`: the rule of the greatest weight among
/// those of [`rules`] or [`rules2`] that match it, from the greatest weight
/// down.
fn rule(path: &str, version2: bool, layout: Layout) -> Rule {
    let name = path.rsplit('/').next().unwrap_or(path);
    // The localized resources: anywhere on iOS, in Resources/ on macOS.
    let localized = match layout {
        Layout::Ios => path,
        Layout::Macos => path.strip_prefix("Resources/").unwrap_or_default(),
    };

    if (version2 && name == ".DS_Store") || localized.ends_with(".lproj/locversion.plist") {
        Rule::Omitted
    } else if localized.starts_with("Base.lproj/") {
        Rule::Sealed
    } else if localized.contains(".lproj/") {
        Rule::Optional
    } else if version2 && (path == "Info.plist" || path == "PkgInfo") {
        Rule::Omitted
    } else if layout == Layout::Ios {
        Rule::Sealed
    } else if !version2 {
        // The first version of macOS seals the resources only.
        if path.starts_with("Resources/") || path == "version.plist" {
            Rule::Sealed
        } else {
            Rule::Omitted
        }
    } else if path.starts_with("Resources/")
        || path == "embedded.provisionprofile"
        || path == "version.plist"
        || path.split('/').any(|part| part.ends_with(".dSYM"))
    {
        Rule::Sealed
    } else if !path.contains('/') || NESTED_CODE.iter().any(|dir| path.starts_with(dir)) {
        Rule::Nested
    } else {
        Rule::Sealed
    }
}

/// Where macOS apps keep nested code, which the second version of their seal
/// does not hash: what codesign writes as
/// `^(Frameworks|SharedFrameworks|PlugIns|Plug-ins|XPCServices|Helpers|MacOS|Library/(Automator|Spotlight|LoginItems))/`.
const NESTED_CODE: &[&str] = &[
    "Frameworks/",
    "SharedFrameworks/",
    "PlugIns/",
    "Plug-ins/",
    "XPCServices/",
    "Helpers/",
    "MacOS/",
    "Library/Automator/",
    "Library/Spotlight/",
    "Library/LoginItems/",
];

/// The rules of the first version, as codesign writes them for the apps of
/// `layout`.
fn rules(layout: Layout) -> Value {
    if layout == Layout::Macos {
        return dictionary([
            ("^Resources/", true.into()),
            ("^Resources/.*\\.lproj/", weighted(1000.0, Some("optional"))),
            (
                "^Resources/.*\\.lproj/locversion.plist$",
                weighted(1100.0, Some("omit")),
            ),
            ("^Resources/Base\\.lproj/", weighted(1010.0, None)),
            ("^version.plist$", true.into()),
        ]);
    }

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

/// The rules of the second version, as codesign writes them for the apps of
/// `layout`.
fn rules2(layout: Layout) -> Value {
    if layout == Layout::Macos {
        return dictionary([
            (".*\\.dSYM($|/)", weighted(11.0, None)),
            ("^(.*/)?\\.DS_Store$", weighted(2000.0, Some("omit"))),
            (
                "^(Frameworks|SharedFrameworks|PlugIns|Plug-ins|XPCServices|Helpers|MacOS|Library/(Automator|Spotlight|LoginItems))/",
                weighted(10.0, Some("nested")),
            ),
            ("^.*", true.into()),
            ("^Info\\.plist$", weighted(20.0, Some("omit"))),
            ("^PkgInfo$", weighted(20.0, Some("omit"))),
            ("^Resources/", weighted(20.0, None)),
            ("^Resources/.*\\.lproj/", weighted(1000.0, Some("optional"))),
            (
                "^Resources/.*\\.lproj/locversion.plist$",
                weighted(1100.0, Some("omit")),
            ),
            ("^Resources/Base\\.lproj/", weighted(1010.0, None)),
            ("^[^/]+$", weighted(10.0, Some("nested"))),
            ("^embedded\\.provisionprofile$", weighted(20.0, None)),
            ("^version\\.plist$", weighted(20.0, None)),
        ]);
    }

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

/// A rule of the given weight, with a flag, `omit`, `optional` or `nested`.
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
    fn rules_as_codesign_applies_them_to_ios_apps() {
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
            assert_eq!(rule(path, false, Layout::Ios), version1, "{path}");
            assert_eq!(rule(path, true, Layout::Ios), version2, "{path}");
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

        let sealed: Dictionary =
            plist::from_bytes(&seal(&app, "Nativella", Layout::Ios).unwrap()).unwrap();

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
        let error = seal(&app, "Nativella", Layout::Ios).unwrap_err();
        assert!(error.to_string().contains("nested code"), "{error}");

        fs::remove_dir_all(app).unwrap();
    }

    #[test]
    fn rules_as_codesign_applies_them_to_macos_apps() {
        for (path, version1, version2) in [
            ("Resources/nativella.bundle", Rule::Sealed, Rule::Sealed),
            ("Resources/AppIcon.icns", Rule::Sealed, Rule::Sealed),
            ("Info.plist", Rule::Omitted, Rule::Omitted),
            ("PkgInfo", Rule::Omitted, Rule::Omitted),
            ("version.plist", Rule::Sealed, Rule::Sealed),
            (
                "Resources/fr.lproj/InfoPlist.strings",
                Rule::Optional,
                Rule::Optional,
            ),
            ("Resources/Base.lproj/Main.nib", Rule::Sealed, Rule::Sealed),
            (
                "Resources/fr.lproj/locversion.plist",
                Rule::Omitted,
                Rule::Omitted,
            ),
            ("Resources/.DS_Store", Rule::Sealed, Rule::Omitted),
            ("MacOS/helper", Rule::Omitted, Rule::Nested),
            ("Frameworks/Lib.framework/Lib", Rule::Omitted, Rule::Nested),
            ("stray.txt", Rule::Omitted, Rule::Nested),
            ("SharedSupport/readme.txt", Rule::Omitted, Rule::Sealed),
        ] {
            assert_eq!(rule(path, false, Layout::Macos), version1, "{path}");
            assert_eq!(rule(path, true, Layout::Macos), version2, "{path}");
        }
    }

    /// The same as codesign: the seal of `testdata/macos-CodeResources.plist`,
    /// which `codesign --force -s -` wrote for an app of these files.
    #[test]
    fn the_seal_of_a_macos_app_as_codesign_writes_it() {
        let contents = Path::new(env!("OUT_DIR"))
            .join(format!("resources-test-macos-{}.app", std::process::id()))
            .join("Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::create_dir_all(contents.join("Resources")).unwrap();
        fs::create_dir_all(contents.join("_CodeSignature")).unwrap();
        fs::write(
            contents.join("MacOS/Reference"),
            [0xcf, 0xfa, 0xed, 0xfe, 0],
        )
        .unwrap();
        fs::write(contents.join("_CodeSignature/CodeResources"), "previous").unwrap();
        fs::write(contents.join("Info.plist"), "info").unwrap();
        fs::write(contents.join("Resources/nativella.bundle"), "bundle").unwrap();
        fs::write(contents.join("Resources/AppIcon.icns"), "icon").unwrap();

        let sealed: Dictionary =
            plist::from_bytes(&seal(&contents, "MacOS/Reference", Layout::Macos).unwrap()).unwrap();

        let reference: Dictionary = plist::from_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/apple/testdata/macos-CodeResources.plist"),
        )
        .unwrap();
        assert_eq!(sealed, reference);

        // Nested code is not sealed, nor files where only code goes.
        fs::write(contents.join("MacOS/notes.txt"), "notes").unwrap();
        let error = seal(&contents, "MacOS/Reference", Layout::Macos).unwrap_err();
        assert!(error.to_string().contains("only code goes"), "{error}");

        fs::remove_dir_all(contents.parent().unwrap()).unwrap();
    }
}
