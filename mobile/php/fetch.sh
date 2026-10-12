#!/bin/sh
# Downloads a static PHP build for Android or iOS: libphp.a, the libraries it
# needs and its headers, into mobile/php/dist/<platform>-<php version>/.
#
# They come from NativePHP's public builds (https://github.com/NativePHP,
# MIT, built with static-php-cli and the NDK / Xcode) until Nativella builds
# its own. Usage: mobile/php/fetch.sh android|ios [php minor version]
set -eu

platform=${1:?Usage: fetch.sh android|ios [8.4|8.5]}
minor=${2:-8.5}
release=4.0.0
# CDPATH would make `cd` print the directory, or pick another one.
dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/dist

manifest=$(curl -sSfL "https://bin.nativephp.com/main/$release.json")
url=$(printf '%s' "$manifest" | php -r '
    $manifest = json_decode(stream_get_contents(STDIN), true, flags: JSON_THROW_ON_ERROR);
    foreach ($manifest["versions"][$argv[1]][$argv[2]] ?? [] as $url) {
        if (!str_ends_with($url, "-icu.zip")) {
            echo $url;
        }
    }
' "$minor" "$platform")
if [ -z "$url" ]; then
    echo "No PHP $minor build for $platform in release $release." >&2
    exit 1
fi

version=$(basename "$url" .zip | sed 's/.*-php//')
target="$dir/$platform-$version"
if [ -d "$target" ]; then
    echo "$target"
    exit 0
fi

mkdir -p "$dir"
curl -sSfL -o "$target.zip" "$url"
mkdir "$target"
unzip -q "$target.zip" -d "$target"
rm "$target.zip"
echo "$target"
