//! The PHP project to run, and how it describes itself: the `extra.nativella`
//! section of its `composer.json`.
//!
//! ```json
//! "extra": {
//!     "nativella": {
//!         "name": "Notes",
//!         "identifier": "dev.nativella.notes",
//!         "version": "1.2.0",
//!         "icon": "assets/icon.png",
//!         "window": { "title": "Notes", "width": 1000, "height": 700 },
//!         "deepLinks": ["notes"],
//!         "permissions": ["haptics"]
//!     }
//! }
//! ```
//!
//! Every key is optional. Keep in sync with the documentation in
//! docs/configuration.md.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A PHP project with a `public/index.php` front controller.
#[derive(Debug, Clone)]
pub struct Project {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

/// What the desktop application is called, and how it starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// The product name: window titles, menus, the autostart entry.
    pub name: String,
    /// A reverse domain name, unique to the application. It names the
    /// directories where the application keeps its data.
    pub identifier: String,
    pub version: String,
    /// A PNG file, relative to the project.
    pub icon: Option<PathBuf>,
    /// The options of the window opened when PHP opens none at boot, as for
    /// `window.open`.
    #[serde(default)]
    pub window: Map<String, Value>,
    /// URL schemes that open the application: `notes` for `notes://…`.
    #[serde(default)]
    pub deep_links: Vec<String>,
    /// The APIs of phones the application uses: mobile builds include them,
    /// and only them.
    #[serde(default)]
    pub permissions: Vec<Permission>,
}

/// An API of phones, which brings permissions to the Android manifest.
///
/// Keep in sync with the features of Cargo.toml, PHONE_DOMAINS in
/// native/mod.rs and the build scripts of mobile/.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Biometrics,
    Geolocation,
    Haptics,
}

/// The `extra.nativella` section as written, before defaults apply.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Section {
    name: Option<String>,
    identifier: Option<String>,
    version: Option<String>,
    icon: Option<PathBuf>,
    #[serde(default)]
    window: Map<String, Value>,
    #[serde(default)]
    deep_links: Vec<String>,
    #[serde(default)]
    permissions: Vec<Permission>,
}

impl Project {
    pub fn load(dir: &Path) -> io::Result<Self> {
        let dir = dir.canonicalize().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "The project directory {} is not readable: {error}",
                    dir.display()
                ),
            )
        })?;

        let composer = dir.join("composer.json");
        let composer: Value = match fs::read(&composer) {
            Ok(content) => serde_json::from_slice(&content).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is not valid JSON: {error}", composer.display()),
                )
            })?,
            // A plain site may have no composer.json at all.
            Err(error) if error.kind() == io::ErrorKind::NotFound => Value::Null,
            Err(error) => return Err(error),
        };

        let section = match composer.pointer("/extra/nativella") {
            Some(section) => Section::deserialize(section).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("The \"extra.nativella\" section of composer.json is invalid: {error}"),
                )
            })?,
            None => Section::default(),
        };

        let manifest = Manifest::from_section(section, &composer, &dir)?;

        Ok(Self { dir, manifest })
    }

    pub fn icon(&self) -> Option<PathBuf> {
        self.manifest.icon.as_ref().map(|icon| self.dir.join(icon))
    }
}

impl Manifest {
    fn from_section(section: Section, composer: &Value, dir: &Path) -> io::Result<Self> {
        // "acme/notes" in composer.json, or the directory name.
        let slug = composer
            .get("name")
            .and_then(Value::as_str)
            .and_then(|name| name.rsplit('/').next())
            .map(str::to_owned)
            .or_else(|| {
                dir.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "app".to_owned());

        let name = section.name.unwrap_or_else(|| title_case(&slug));
        let identifier = section
            .identifier
            .unwrap_or_else(|| format!("dev.nativella.{}", identifier_segment(&slug)));
        validate_identifier(&identifier)?;

        let version = section
            .version
            .or_else(|| {
                composer
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "0.1.0".to_owned());

        for scheme in &section.deep_links {
            validate_scheme(scheme)?;
        }

        Ok(Self {
            name,
            identifier,
            version,
            icon: section.icon,
            window: section.window,
            deep_links: section.deep_links,
            permissions: section.permissions,
        })
    }
}

/// The identifier names directories: keep it to what a reverse domain name
/// may contain, so that it can never escape them.
fn validate_identifier(identifier: &str) -> io::Result<()> {
    let valid = !identifier.is_empty()
        && identifier.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|char| char.is_ascii_alphanumeric() || char == '-')
        });

    if valid {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "The identifier \"{identifier}\" is not a reverse domain name (letters, digits and dashes, separated by dots)."
        ),
    ))
}

/// RFC 3986: a letter, then letters, digits, `+`, `-` and `.`. The web ones
/// would hand every link of the system to the application.
fn validate_scheme(scheme: &str) -> io::Result<()> {
    let valid = scheme.starts_with(|char: char| char.is_ascii_lowercase())
        && scheme
            .chars()
            .all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || "+-.".contains(char))
        && !matches!(scheme, "file" | "http" | "https" | "mailto");

    if valid {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "\"{scheme}\" cannot be a deep link scheme: use lowercase letters, digits, \"+\", \"-\" and \".\", starting with a letter, and not a web scheme."
        ),
    ))
}

fn identifier_segment(slug: &str) -> String {
    let segment: String = slug
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let segment = segment.trim_matches('-');

    if segment.is_empty() {
        "app".to_owned()
    } else {
        segment.to_owned()
    }
}

fn title_case(slug: &str) -> String {
    slug.split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest(composer: Value) -> io::Result<Manifest> {
        let section = match composer.pointer("/extra/nativella") {
            Some(section) => Section::deserialize(section).unwrap(),
            None => Section::default(),
        };

        Manifest::from_section(section, &composer, Path::new("/projects/my-site"))
    }

    #[test]
    fn defaults_come_from_the_package_name() {
        let manifest = manifest(json!({ "name": "acme/note-taker" })).unwrap();

        assert_eq!(manifest.name, "Note Taker");
        assert_eq!(manifest.identifier, "dev.nativella.note-taker");
        assert_eq!(manifest.version, "0.1.0");
    }

    #[test]
    fn defaults_come_from_the_directory_without_composer_json() {
        let manifest = manifest(Value::Null).unwrap();

        assert_eq!(manifest.name, "My Site");
        assert_eq!(manifest.identifier, "dev.nativella.my-site");
    }

    #[test]
    fn the_section_wins() {
        let manifest = manifest(json!({
            "name": "acme/notes",
            "version": "2.0.0",
            "extra": { "nativella": {
                "name": "Acme Notes",
                "identifier": "com.acme.notes",
                "window": { "width": 400 },
            } },
        }))
        .unwrap();

        assert_eq!(manifest.name, "Acme Notes");
        assert_eq!(manifest.identifier, "com.acme.notes");
        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.window.get("width"), Some(&json!(400)));
    }

    #[test]
    fn deep_link_schemes_are_checked() {
        let parsed = manifest(
            json!({ "extra": { "nativella": { "deepLinks": ["notes", "acme.notes+v2"] } } }),
        )
        .unwrap();
        assert_eq!(parsed.deep_links, ["notes", "acme.notes+v2"]);

        for scheme in ["https", "Notes", "2notes", "no tes", ""] {
            let error = manifest(json!({ "extra": { "nativella": { "deepLinks": [scheme] } } }))
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{scheme}");
        }
    }

    #[test]
    fn rejects_an_identifier_that_is_not_a_domain_name() {
        for identifier in ["", "com..acme", "../../etc", "com/acme", "com.acme."] {
            let error = manifest(json!({ "extra": { "nativella": { "identifier": identifier } } }))
                .unwrap_err();

            assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{identifier}");
        }
    }
}
