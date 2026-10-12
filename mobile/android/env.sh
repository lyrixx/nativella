# shellcheck shell=bash
# Environment of an Android build: source it, then run `cargo tauri android …`
# from crates/nativella (see mobile/README.md).
#
# Expects ANDROID_HOME (the SDK, with an NDK installed) and JAVA_HOME (JDK 17+).

if [ -z "${ANDROID_HOME:-}" ] || [ -z "${JAVA_HOME:-}" ]; then
    echo "Set ANDROID_HOME to the Android SDK and JAVA_HOME to a JDK 17+ first." >&2
    return 1
fi

nativella_mobile=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]:-$0}")/.." && pwd)

# The most recent NDK of the SDK, unless NDK_HOME says otherwise.
export NDK_HOME=${NDK_HOME:-$(printf '%s\n' "$ANDROID_HOME"/ndk/* | sort -V | tail -n 1)}

# PHP for Android: libphp.a and its headers, the stand-ins describing it to
# ext-php-rs, and the NDK system headers for its bindings.
export NATIVELLA_PHP_DIR=${NATIVELLA_PHP_DIR:-$("$nativella_mobile/php/fetch.sh" android)}
export NATIVELLA_PHP_LIB_DIR="$NATIVELLA_PHP_DIR/staticLibs/arm64-v8a"
export PHP="$nativella_mobile/php/php"
export PHP_CONFIG="$nativella_mobile/php/php-config"
# PHP's headers use GNU extensions of bionic (mempcpy), as PHP itself was built.
export CFLAGS_aarch64_linux_android="-D_GNU_SOURCE"
# bindgen targets the API level 24 too, the first with all of them.
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android="--target=aarch64-linux-android24 --sysroot=$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot -D_GNU_SOURCE"

unset nativella_mobile
