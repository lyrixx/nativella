//! Links PHP statically when `NATIVELLA_PHP_LIB_DIR` points to a directory
//! holding `libphp.a` and the static libraries it needs: always on mobile,
//! where there is no system PHP, and on the desktop for applications that
//! must run without PHP installed (castor static-php:build, see
//! docs/development.md).
//!
//! ext-php-rs already asks for `-lphp`: without that variable, desktop builds
//! link the libphp of the system.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=NATIVELLA_PHP_LIB_DIR");
    println!("cargo:rustc-check-cfg=cfg(nativella_static_php)");

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Cargo sets the target OS.");
    let mobile = target_os == "android" || target_os == "ios";

    let lib_dir = match env::var("NATIVELLA_PHP_LIB_DIR") {
        Ok(lib_dir) => lib_dir,
        Err(_) if mobile => panic!(
            "Building for {target_os} needs NATIVELLA_PHP_LIB_DIR: the directory of a static libphp.a, see docs/mobile.md."
        ),
        Err(_) => return,
    };
    let lib_dir = Path::new(&lib_dir);
    assert!(
        lib_dir.join("libphp.a").is_file(),
        "{} holds no libphp.a.",
        lib_dir.display()
    );
    println!("cargo:rerun-if-changed={}", lib_dir.display());

    println!("cargo:rustc-cfg=nativella_static_php");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=php");

    // Every other static library of the build: they are the dependencies of
    // its extensions, and the linker only takes what libphp uses.
    let mut libraries: Vec<String> = fs::read_dir(lib_dir)
        .unwrap_or_else(|error| panic!("Cannot list {}: {error}", lib_dir.display()))
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            let library = name.strip_prefix("lib")?.strip_suffix(".a")?;

            (library != "php").then(|| library.to_owned())
        })
        .collect();
    libraries.sort();
    for library in libraries {
        println!("cargo:rustc-link-lib=static={library}");
    }

    let system: &[&str] = match target_os.as_str() {
        "android" => &["dl", "log", "m"],
        "linux" => &["dl", "m", "pthread", "resolv", "rt", "util"],
        "ios" | "macos" => {
            for framework in ["CoreFoundation", "Security", "SystemConfiguration"] {
                println!("cargo:rustc-link-lib=framework={framework}");
            }
            &["resolv"]
        }
        _ => &[],
    };
    for library in system {
        println!("cargo:rustc-link-lib={library}");
    }
}
