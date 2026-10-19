# Nativella for Symfony

Runs a Symfony application as a desktop or mobile application with [Nativella](https://lyrixx.github.io/nativella/). The
bundle registers the native APIs as services, dispatches native events through the event dispatcher, migrates the
database when the application starts, and adds a Nativella panel to the profiler.

```bash
composer require nativella/symfony-bundle
```

There is no Flex recipe yet: enable the bundle yourself.

```php
// config/bundles.php
return [
    // ...
    Nativella\Symfony\NativellaBundle::class => ['all' => true],
];
```

The application runs with the `nativella` command: see [Installation](https://lyrixx.github.io/nativella/docs/installation/)
and [Symfony](https://lyrixx.github.io/nativella/docs/symfony/).

## Contributing

This bundle is developed in [`packages/symfony`](https://github.com/lyrixx/nativella/tree/main/packages/symfony) of the
[Nativella repository](https://github.com/lyrixx/nativella);
[lyrixx/nativella-symfony-bundle](https://github.com/lyrixx/nativella-symfony-bundle) is a read-only copy of it. Report
issues and send pull requests to the Nativella repository.

## License

MIT, see [LICENSE](LICENSE).
