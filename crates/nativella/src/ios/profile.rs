//! A provisioning profile (`.mobileprovision`), from the Apple Developer
//! website: what lets an app run on devices, the ones it lists for
//! development, any for the App Store. A property list, in a CMS signature of
//! Apple's, which iOS checks: the app embeds it as is.
//!
//! The app gets the entitlements of the profile, as re-signing tools do:
//! its application identifier, for the bundle ID of the app when the profile
//! is for a wildcard App ID, its team, whether debuggers may attach
//! (`get-task-allow`, in development), and the capabilities of the App ID.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use cms::content_info::ContentInfo;
use cms::signed_data::SignedData;
use der::Decode;
use der::asn1::{ObjectIdentifier, OctetString};
use plist::{Dictionary, Value};

use super::certificate::Certificate;

const ID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");

/// The key of the entitlements that names the app the profile is for:
/// `<prefix>.<bundle ID>`, the bundle ID possibly ending with `*`.
const APPLICATION_IDENTIFIER: &str = "application-identifier";

pub struct Profile {
    path: PathBuf,
    name: String,
    team: String,
    entitlements: Dictionary,
    /// The certificates that may sign with it, in DER.
    certificates: Vec<Vec<u8>>,
}

impl Profile {
    pub fn load(path: &Path) -> io::Result<Self> {
        let data = fs::read(path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Cannot read the provisioning profile {}: {error}.",
                    path.display()
                ),
            )
        })?;
        let not_a_profile = || {
            invalid(&format!(
                "{} is not a provisioning profile (.mobileprovision).",
                path.display()
            ))
        };

        let content = ContentInfo::from_der(&data).map_err(|_| not_a_profile())?;
        if content.content_type != ID_SIGNED_DATA {
            return Err(not_a_profile());
        }
        let profile = content
            .content
            .decode_as::<SignedData>()
            .map_err(|_| not_a_profile())?
            .encap_content_info
            .econtent
            .ok_or_else(not_a_profile)?
            .decode_as::<OctetString>()
            .map_err(|_| not_a_profile())?;
        let profile: Dictionary =
            plist::from_bytes(profile.as_bytes()).map_err(|_| not_a_profile())?;

        Self::of(path, &profile)
    }

    /// The profile `path` holds, read.
    fn of(path: &Path, profile: &Dictionary) -> io::Result<Self> {
        let invalid_profile = |what: &str| {
            invalid(&format!(
                "The provisioning profile {} has no {what}.",
                path.display()
            ))
        };
        let name = profile
            .get("Name")
            .and_then(Value::as_string)
            .unwrap_or_default()
            .to_owned();

        let platforms: Vec<&str> = profile
            .get("Platform")
            .and_then(Value::as_array)
            .map(|platforms| platforms.iter().filter_map(Value::as_string).collect())
            .unwrap_or_default();
        if !platforms.contains(&"iOS") {
            return Err(invalid(&format!(
                "The provisioning profile {} is for {}, not for iOS.",
                path.display(),
                platforms.join(", ")
            )));
        }
        let expiry = profile
            .get("ExpirationDate")
            .and_then(Value::as_date)
            .ok_or_else(|| invalid_profile("expiration date"))?;
        if SystemTime::from(expiry) < SystemTime::now() {
            return Err(invalid(&format!(
                "The provisioning profile {} expired on {}: download it again from the Apple Developer website.",
                path.display(),
                expiry.to_xml_format()
            )));
        }

        let team = profile
            .get("TeamIdentifier")
            .and_then(Value::as_array)
            .and_then(|teams| teams.first())
            .and_then(Value::as_string)
            .ok_or_else(|| invalid_profile("team"))?
            .to_owned();
        let entitlements = profile
            .get("Entitlements")
            .and_then(Value::as_dictionary)
            .ok_or_else(|| invalid_profile("entitlements"))?
            .clone();
        let certificates = profile
            .get("DeveloperCertificates")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid_profile("certificates"))?
            .iter()
            .filter_map(Value::as_data)
            .map(<[u8]>::to_vec)
            .collect();

        Ok(Self {
            path: path.to_owned(),
            name,
            team,
            entitlements,
            certificates,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Checks that `certificate` may sign with the profile: that the profile
    /// lists it, which makes it of the same team.
    pub fn check(&self, certificate: &Certificate) -> io::Result<()> {
        if !self.certificates.contains(&certificate.der()?) {
            return Err(invalid(&format!(
                "The provisioning profile {} (\"{}\") does not list the certificate {} (\"{}\"): download a profile made with it from the Apple Developer website.",
                self.path.display(),
                self.name,
                certificate.path().display(),
                certificate.common_name()
            )));
        }
        if certificate.team() != self.team {
            return Err(invalid(&format!(
                "The provisioning profile {} is for the team {}, the certificate {} for the team {}.",
                self.path.display(),
                self.team,
                certificate.path().display(),
                certificate.team()
            )));
        }

        Ok(())
    }

    /// The entitlements of the app `identifier`: those of the profile, with
    /// its application identifier, when the profile is for it.
    pub fn entitlements(&self, identifier: &str) -> io::Result<Dictionary> {
        let application_identifier = self
            .entitlements
            .get(APPLICATION_IDENTIFIER)
            .and_then(Value::as_string)
            .ok_or_else(|| {
                invalid(&format!(
                    "The provisioning profile {} has no application identifier.",
                    self.path.display()
                ))
            })?;
        let (prefix, app_id) = application_identifier.split_once('.').ok_or_else(|| {
            invalid(&format!(
                "The application identifier {application_identifier} of the provisioning profile {} has no prefix.",
                self.path.display()
            ))
        })?;

        // Bundle IDs ignore case.
        let identifier_lowercase = identifier.to_ascii_lowercase();
        let app_id_lowercase = app_id.to_ascii_lowercase();
        let matches = match app_id_lowercase.strip_suffix('*') {
            Some(start) => identifier_lowercase.starts_with(start),
            None => identifier_lowercase == app_id_lowercase,
        };
        if !matches {
            return Err(invalid(&format!(
                "The provisioning profile {} is for the App ID {app_id}, not for {identifier}: set extra.nativella.identifier in composer.json to it, or download a profile for {identifier} from the Apple Developer website.",
                self.path.display()
            )));
        }

        let mut entitlements = self.entitlements.clone();
        entitlements.insert(
            APPLICATION_IDENTIFIER.to_owned(),
            format!("{prefix}.{identifier}").into(),
        );

        Ok(entitlements)
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ios::certificate::tests::{PASSWORD, testdata};

    fn profile() -> Profile {
        Profile::load(&testdata("development.mobileprovision")).unwrap()
    }

    #[test]
    fn a_profile_lists_the_certificates_that_sign_with_it() {
        let profile = profile();
        let certificate = Certificate::load(&testdata("developer.p12"), PASSWORD).unwrap();

        assert_eq!(profile.name, "Nativella Tests");
        assert_eq!(profile.team, "TEAM123456");
        profile.check(&certificate).unwrap();

        let other = Profile {
            certificates: Vec::new(),
            ..profile
        };
        let error = other.check(&certificate).unwrap_err();
        assert!(error.to_string().contains("does not list"), "{error}");
    }

    #[test]
    fn the_entitlements_of_an_app_it_is_for() {
        let entitlements = profile().entitlements("dev.nativella.Notes").unwrap();

        assert_eq!(
            entitlements[APPLICATION_IDENTIFIER].as_string(),
            Some("TEAM123456.dev.nativella.Notes")
        );
        assert_eq!(entitlements["get-task-allow"].as_boolean(), Some(true));
        assert_eq!(
            entitlements["com.apple.developer.team-identifier"].as_string(),
            Some("TEAM123456")
        );

        let error = profile().entitlements("com.acme.notes").unwrap_err();
        assert!(error.to_string().contains("dev.nativella.*"), "{error}");
    }

    #[test]
    fn an_expired_profile_or_one_of_another_platform() {
        let path = Path::new("old.mobileprovision");
        let mut profile = Dictionary::new();
        profile.insert("Platform".to_owned(), vec![Value::from("iOS")].into());
        profile.insert(
            "ExpirationDate".to_owned(),
            Value::Date(SystemTime::UNIX_EPOCH.into()),
        );
        let error = Profile::of(path, &profile).err().unwrap();
        assert!(error.to_string().contains("expired"), "{error}");

        profile.insert("Platform".to_owned(), vec![Value::from("OSX")].into());
        let error = Profile::of(path, &profile).err().unwrap();
        assert!(error.to_string().contains("not for iOS"), "{error}");
    }
}
