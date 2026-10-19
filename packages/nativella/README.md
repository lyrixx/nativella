# Nativella for PHP

The PHP API of [Nativella](https://lyrixx.github.io/nativella/), which runs a PHP web application as a desktop or
mobile application: windows, menus, tray icon, dialogs, notifications, clipboard, global shortcuts, settings,
secrets… the native events, and a fake client for tests. It needs no framework; the
[Symfony](https://github.com/lyrixx/nativella-symfony-bundle) and [Laravel](https://github.com/lyrixx/nativella-laravel)
integrations build on it.

```bash
composer require nativella/nativella
```

```php
use Nativella\Nativella;

$native = Nativella::create();

$native->window->setTitle('Snippets');
$native->notification->send('Copied', 'The snippet is in the clipboard.');
```

The application runs with the `nativella` command: see [Installation](https://lyrixx.github.io/nativella/docs/installation/)
and [Plain PHP](https://lyrixx.github.io/nativella/docs/plain-php/).

## Contributing

This package is developed in [`packages/nativella`](https://github.com/lyrixx/nativella/tree/main/packages/nativella) of the
[Nativella repository](https://github.com/lyrixx/nativella); [lyrixx/nativella-php](https://github.com/lyrixx/nativella-php)
is a read-only copy of it. Report issues and send pull requests to the Nativella repository.

## License

MIT, see [LICENSE](LICENSE).
