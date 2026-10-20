# Mobile builds

What the mobile builds need besides their Castor tasks, `castor mobile:android-template` and `castor mobile:ios` (`.castor/mobile.php`), and the builds of PHP for phones, `castor static-php:android` and `castor static-php:ios` (`.castor/static-php-mobile.php`). How they work and how to use them: [docs/mobile.md](../docs/mobile.md).

| Path | What |
|---|---|
| `php/php` | Describes a PHP build without a `php` binary to ext-php-rs, which runs it in place of the PHP of the computer: the PHP of a phone, or the static PHP of `castor static-php:build`. |
| `pom-licenses.php` | Reads the licenses the POMs of the Java libraries of the template APK declare, for its third-party notices (`.castor/notices.php`), with the PHP of the builder: the static binary of Castor, which the CI runs, has no XML extension. |
