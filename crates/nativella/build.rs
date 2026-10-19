//! `NATIVELLA_BUNDLE` names a bundle written by `nativella bundle`: iOS
//! builds compile it into the library (see src/embedded.rs). Android ones
//! carry it in their APK instead.
//!
//! `NATIVELLA_NOTICES` names the third-party notices of the build, which
//! `castor nativella:build` writes: they are compressed into the executable
//! (see src/notices.rs).

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use flate2::Compression;
use flate2::write::GzEncoder;

fn main() {
    println!("cargo:rerun-if-env-changed=NATIVELLA_BUNDLE");
    println!("cargo:rerun-if-env-changed=NATIVELLA_NOTICES");

    let out_dir = env::var("OUT_DIR").expect("Cargo sets OUT_DIR.");
    let out_dir = Path::new(&out_dir);

    let target = out_dir.join("bundle.bin");
    match env::var("NATIVELLA_BUNDLE") {
        Ok(bundle) => {
            println!("cargo:rerun-if-changed={bundle}");
            fs::copy(&bundle, &target)
                .unwrap_or_else(|error| panic!("Cannot read the bundle {bundle}: {error}"));
        }
        Err(_) => fs::write(&target, []).expect("OUT_DIR is writable."),
    }

    let target = out_dir.join("notices.txt.gz");
    match env::var("NATIVELLA_NOTICES") {
        Ok(notices) => {
            println!("cargo:rerun-if-changed={notices}");
            let text = fs::read(&notices)
                .unwrap_or_else(|error| panic!("Cannot read the notices {notices}: {error}"));
            let mut encoder = GzEncoder::new(
                File::create(&target).expect("OUT_DIR is writable."),
                Compression::best(),
            );
            encoder.write_all(&text).expect("OUT_DIR is writable.");
            encoder.finish().expect("OUT_DIR is writable.");
        }
        // A development build: `nativella licenses` says it has none.
        Err(_) => fs::write(&target, []).expect("OUT_DIR is writable."),
    }

    // The capabilities are in tauri.conf.json. Without a pattern of its own,
    // tauri-build watches a capabilities/ directory, which does not exist: a
    // missing watched path is always dirty, and cargo compiled the crate again
    // at every build.
    tauri_build::try_build(
        tauri_build::Attributes::new().capabilities_path_pattern("capabilities/**/*"),
    )
    .unwrap_or_else(|error| panic!("{error:#}"));
}
