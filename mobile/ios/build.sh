#!/bin/bash
# Builds an iOS application of a PHP project of this repository, for the
# simulator (arm64), as mobile/android/build.sh does for Android.
#
# Usage: mobile/ios/build.sh demos/symfony [--release]
#
# EXPERIMENTAL, NEVER RUN: it needs macOS, Xcode and XcodeGen, which the
# repository is not developed on. .github/workflows/mobile.yml runs it on
# macOS, on demand, as an experiment.
#
# NATIVELLA_BUNDLE, when set, names a bundle made beforehand (`nativella
# bundle`, e.g. on Linux). Otherwise the project is staged and bundled here,
# with the PHP, Composer and nativella command of the Mac, the last one
# linking a libphp of the Mac.
set -euo pipefail

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
project=${1:?Usage: build.sh <project-dir> [--release]}
profile=--debug
[ "${2:-}" = "--release" ] && profile=

if [ -n "${NATIVELLA_BUNDLE:-}" ]; then
    staged=$project
else
    staged=$("$root/mobile/stage.sh" "$project")
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p nativella -- bundle "$staged" -o "$root/target/nativella/app.bundle"
    export NATIVELLA_BUNDLE="$root/target/nativella/app.bundle"
fi

# PHP for iOS: libphp.a and the libraries it needs for the simulator, its
# headers, and the stand-ins describing it to ext-php-rs (see mobile/php/).
export NATIVELLA_PHP_DIR=${NATIVELLA_PHP_DIR:-$("$root/mobile/php/fetch.sh" ios)}
export NATIVELLA_PHP_LIB_DIR="$NATIVELLA_PHP_DIR/Libraries/iphonesimulator"
export PHP="$root/mobile/php/php"
export PHP_CONFIG="$root/mobile/php/php-config"
# bindgen parses PHP's headers for the simulator, with its SDK.
sdk=$(xcrun --sdk iphonesimulator --show-sdk-path)
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_ios_sim="--target=arm64-apple-ios-simulator -isysroot $sdk"

# The identity of the application, from its composer.json, overrides the
# placeholder of tauri.conf.json.
config=$(php -r '
    $composer = json_decode(file_get_contents($argv[1]."/composer.json"), true, flags: JSON_THROW_ON_ERROR);
    $nativella = $composer["extra"]["nativella"] ?? [];
    echo json_encode(array_filter([
        "identifier" => $nativella["identifier"] ?? null,
        "productName" => $nativella["name"] ?? null,
        "version" => $nativella["version"] ?? null,
    ]), JSON_THROW_ON_ERROR);
' "$staged")
# The APIs of phones the application asks for, and only them: each one is a
# feature of the nativella crate, which brings its permissions.
features=$(php -r '
    $composer = json_decode(file_get_contents($argv[1]."/composer.json"), true, flags: JSON_THROW_ON_ERROR);
    echo implode(",", $composer["extra"]["nativella"]["permissions"] ?? []);
' "$staged")

# iOS asks the user for a permission only with a reason, from Info.plist:
# Tauri merges this file into it.
php -r '
    $composer = json_decode(file_get_contents($argv[1]."/composer.json"), true, flags: JSON_THROW_ON_ERROR);
    $nativella = $composer["extra"]["nativella"] ?? [];
    $name = $nativella["name"] ?? basename($argv[1]);
    $reasons = [
        "biometrics" => ["NSFaceIDUsageDescription" => "$name uses Face ID to check that it is you."],
        "geolocation" => ["NSLocationWhenInUseUsageDescription" => "$name uses the location of the phone."],
    ];
    $keys = "";
    foreach ($nativella["permissions"] ?? [] as $permission) {
        foreach ($reasons[$permission] ?? [] as $key => $reason) {
            $keys .= "    <key>$key</key>\n    <string>".htmlspecialchars($reason, ENT_XML1)."</string>\n";
        }
    }
    echo "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
    echo "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n";
    echo "<plist version=\"1.0\">\n<dict>\n$keys</dict>\n</plist>\n";
' "$staged" > "$root/crates/nativella/Info.ios.plist"

cd "$root/crates/nativella"
identifier=$(php -r 'echo json_decode($argv[1], true)["identifier"] ?? "";' "$config")
# The Xcode project is named after the application: generate it again when
# the identifier changes.
if [ ! -f gen/apple/.nativella-identifier ] || [ "$(cat gen/apple/.nativella-identifier)" != "$identifier" ]; then
    rm -rf gen/apple
    cargo tauri ios init --ci --config "$config"
    echo "$identifier" > gen/apple/.nativella-identifier
fi

# A simulator build needs no signing.
cargo tauri ios build --ci --target aarch64-sim --no-sign $profile --config "$config" ${features:+--features "$features"}
find gen/apple/build -name '*.app' -prune
