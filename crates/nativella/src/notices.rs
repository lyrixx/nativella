//! The third-party notices: the copyright notices and the licenses of what
//! the build contains, which most of those licenses ask to ship with it.
//!
//! `castor nativella:build` gathers them (.castor/notices.php) and build.rs
//! compresses them into the executable, and so into every application
//! `nativella build` makes of it; `castor mobile:ios-template` into the
//! library of the template app, and so into every iOS app `nativella ios`
//! makes of it. An Android application carries them as an asset of its APK
//! instead, from the template.

use std::io::{self, Read};

/// The name of the notices: next to the executables `nativella build`
/// writes, and an asset of the APKs.
pub const FILE: &str = "THIRD_PARTY_NOTICES.txt";

/// The notices, gzipped, empty in a development build.
#[cfg(not(target_os = "android"))]
static COMPRESSED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/notices.txt.gz"));

/// The notices of the build, `None` for a development build, which has none.
#[cfg(not(target_os = "android"))]
pub fn text() -> io::Result<Option<String>> {
    if COMPRESSED.is_empty() {
        return Ok(None);
    }

    let mut text = String::new();
    flate2::read::GzDecoder::new(COMPRESSED).read_to_string(&mut text)?;

    Ok(Some(text))
}

/// The notices of the application: the asset [`FILE`] of its APK.
#[cfg(target_os = "android")]
pub fn text() -> io::Result<Option<String>> {
    let Some(mut asset) = crate::embedded::asset::open(FILE)? else {
        return Ok(None);
    };

    let mut text = String::new();
    asset.read_to_string(&mut text)?;

    Ok(Some(text))
}
