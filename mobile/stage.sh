#!/bin/bash
# Prepares a PHP project of this repository for a production bundle, in
# target/nativella/stage/: dependencies without dev packages (copied, not
# symlinked) and the production environment. Prints the staged directory.
#
# Usage: mobile/stage.sh demos/symfony
set -euo pipefail

root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
project=$(CDPATH='' cd -- "$1" && pwd)
relative=${project#"$root"/}
stage="$root/target/nativella/stage"

rm -rf "${stage:?}/${relative:?}"
mkdir -p "$stage/$relative"
# The packages, for the path repositories of the demos.
rsync -a --delete --exclude vendor --exclude composer.lock "$root/packages/" "$stage/packages/"
# Not bootstrap/cache/*.php either: Laravel's package discovery of the dev
# install, which lists dev packages.
rsync -a --exclude vendor --exclude node_modules --exclude 'var/' --exclude 'storage/logs/*' \
    --exclude 'bootstrap/cache/*.php' --exclude '.env.local' --exclude '.env.*.local' --exclude tests \
    "$project/" "$stage/$relative/"

cd "$stage/$relative"
COMPOSER_MIRROR_PATH_REPOS=1 composer install --no-dev --optimize-autoloader --no-interaction --no-progress --no-scripts --quiet >&2

if [ -f bin/console ]; then
    # Symfony: the compiled .env of production. The cache is built on the
    # device: it holds absolute paths.
    composer dump-env prod --quiet >&2
elif [ -f artisan ]; then
    # Laravel. A fresh clone has no .env yet. PHP rather than `sed -i`,
    # whose syntax differs on macOS.
    [ -f .env ] || cp .env.example .env
    php artisan package:discover --quiet >&2
    php -r '
        $env = file_get_contents(".env");
        $env = preg_replace(["/^APP_ENV=.*$/m", "/^APP_DEBUG=.*$/m"], ["APP_ENV=production", "APP_DEBUG=false"], $env);
        file_put_contents(".env", $env);
    '
fi

echo "$stage/$relative"
