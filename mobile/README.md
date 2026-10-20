# Mobile builds

What the mobile builds need besides their Castor tasks, `castor mobile:android-template` and `castor mobile:ios-template` (`.castor/mobile.php`), and the builds of PHP for phones, `castor static-php:android` and `castor static-php:ios` (`.castor/static-php-mobile.php`). How they work and how to use them: [docs/mobile.md](../docs/mobile.md).

| Path | What |
|---|---|
| `php/php` | Describes a PHP build without a `php` binary to ext-php-rs, which runs it in place of the PHP of the computer: the PHP of a phone, or the static PHP of `castor static-php:build`. |
