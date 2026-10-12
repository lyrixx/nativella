//! `NATIVELLA_BUNDLE` names a bundle written by `nativella bundle`: mobile
//! builds compile it into the library (see src/embedded.rs).

use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-env-changed=NATIVELLA_BUNDLE");

    let target = Path::new(&env::var("OUT_DIR").expect("Cargo sets OUT_DIR.")).join("bundle.bin");
    match env::var("NATIVELLA_BUNDLE") {
        Ok(bundle) => {
            println!("cargo:rerun-if-changed={bundle}");
            fs::copy(&bundle, &target)
                .unwrap_or_else(|error| panic!("Cannot read the bundle {bundle}: {error}"));
        }
        Err(_) => fs::write(&target, []).expect("OUT_DIR is writable."),
    }

    tauri_build::build();
}
