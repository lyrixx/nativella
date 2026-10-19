# Mobile builds

What the mobile builds need besides their Castor tasks, `castor mobile:android-template` and `castor mobile:ios` (`.castor/mobile.php`). How they work and how to use them: [docs/mobile.md](../docs/mobile.md).

| Path | What |
|---|---|
| `php/php`, `php/php-config` | Describe a PHP build without a `php` binary to ext-php-rs, which runs them in place of the PHP of the computer: the PHP of a phone, or the static PHP of `castor static-php:build`. |
| `php/dist/` | The PHP builds for phones that the tasks download (ignored by Git). |
