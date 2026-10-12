# shellcheck shell=bash
# Environment of a build that links the static PHP of static-php/build.sh:
# source it, then `cargo build --release -p nativella`. Use another target
# directory (--target-dir target/static) to keep the builds linked to the
# system PHP apart.

nativella_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]:-$0}")/.." && pwd)
buildroot="$nativella_root/static-php/buildroot"

if [ ! -f "$buildroot/lib/libphp.a" ]; then
    echo "No static PHP in $buildroot: run static-php/build.sh first." >&2
    unset nativella_root buildroot
    return 1
fi

# The runtime links every static library of lib/ (crates/runtime/build.rs).
export NATIVELLA_PHP_LIB_DIR="$buildroot/lib"
# ext-php-rs reads `php -i` and php-config; an embed-only build has no php
# binary: the stand-in of mobile/php describes it from its headers.
export NATIVELLA_PHP_DIR="$buildroot"
export PHP="$nativella_root/mobile/php/php"
export PHP_CONFIG="$buildroot/bin/php-config"

unset nativella_root buildroot
