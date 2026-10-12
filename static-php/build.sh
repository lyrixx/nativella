#!/bin/bash
# Builds PHP as a static library (embed SAPI) with static-php-cli, for
# `nativella` executables that run without PHP installed: see README.md.
#
# Usage: static-php/build.sh [<php version>]   (default: 8.5)
# The result lands in static-php/buildroot/: lib/libphp.a and its
# dependencies, include/php/, bin/php-config.
set -euo pipefail
unset CDPATH

php_version=${1:-8.5}
spc_version=2.8.5
dir=$(cd -- "$(dirname -- "$0")" && pwd)
cd "$dir"

# The extensions Symfony, Laravel and Doctrine applications need. Keep in
# sync with docs/building.md.
extensions="bcmath,ctype,curl,dom,fileinfo,filter,iconv,mbstring,opcache,openssl,pdo,pdo_sqlite,phar,session,simplexml,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,zip,zlib"

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) asset=spc-linux-x86_64.tar.gz ;;
    Linux-aarch64) asset=spc-linux-aarch64.tar.gz ;;
    Darwin-arm64) asset=spc-macos-aarch64.tar.gz ;;
    Darwin-x86_64) asset=spc-macos-x86_64.tar.gz ;;
    *) echo "static-php-cli has no build for $(uname -s) $(uname -m)." >&2; exit 1 ;;
esac

if [ ! -x spc ]; then
    curl -sSfL "https://github.com/crazywhalecc/static-php-cli/releases/download/$spc_version/$asset" | tar xz
fi

# Linked against the glibc of the system, as the rest of the application
# (WebKitGTK) is: static-php-cli builds with zig for that.
if [ "$(uname -s)" = Linux ]; then
    export SPC_TARGET=native-native-gnu
fi

./spc doctor --auto-fix
./spc download --for-extensions="$extensions" --with-php="$php_version" --prefer-pre-built
./spc build "$extensions" --build-embed

echo "Static PHP ready in $dir/buildroot"
