#!/bin/bash
# Builds an Android APK (arm64) of a PHP project of this repository.
#
# Usage: ANDROID_HOME=… JAVA_HOME=… mobile/android/build.sh demos/symfony [--release]
set -euo pipefail

root=$(CDPATH='' cd -- "$(dirname -- "$0")/../.." && pwd)
project=${1:?Usage: build.sh <project-dir> [--release]}
profile=--debug
[ "${2:-}" = "--release" ] && profile=

# Bundled by the nativella command of the host, before the Android
# environment points PHP to the Android build.
staged=$("$root/mobile/stage.sh" "$project")
cargo run --quiet --manifest-path "$root/Cargo.toml" -p nativella -- bundle "$staged" -o "$root/target/nativella/app.bundle"
export NATIVELLA_BUNDLE="$root/target/nativella/app.bundle"

# shellcheck source=env.sh
source "$root/mobile/android/env.sh"

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

cd "$root/crates/nativella"
identifier=$(php -r 'echo json_decode($argv[1], true)["identifier"] ?? "";' "$config")
# The Android project names its package after the identifier: generate it
# again when the identifier changes.
if [ ! -f gen/android/.nativella-identifier ] || [ "$(cat gen/android/.nativella-identifier)" != "$identifier" ]; then
    rm -rf gen/android
    cargo tauri android init --ci --config "$config"
    echo "$identifier" > gen/android/.nativella-identifier
fi

cargo tauri android build --ci --target aarch64 --apk $profile --config "$config" ${features:+--features "$features"}
find gen/android/app/build/outputs/apk -name '*.apk' -newer "$NATIVELLA_BUNDLE"
