# Static PHP

The working directory of the tasks that build PHP as a static library:

- `castor static-php:build` (`.castor/static-php.php`), for the desktop, with [static-php-cli](https://github.com/crazywhalecc/static-php-cli): the tool, its downloads and sources, and the result, `buildroot/`;
- `castor static-php:android` and `castor static-php:ios` (`.castor/static-php-mobile.php`), for the phones, from the same downloads: `android-arm64/`, `ios-arm64/`… each with its `buildroot/`.

Everything here but this file is ignored by Git.

How to use it: [docs/development.md](../docs/development.md#static-php).
