# Nativella for Laravel

Runs a Laravel application as a desktop or mobile application with [Nativella](https://lyrixx.github.io/nativella/). The
package binds the native APIs in the container and gives them facades, dispatches native events as Laravel events, keeps
the SQLite database in the data directory of the application and migrates it when the application starts.

```bash
composer require nativella/laravel
```

Package discovery registers `Nativella\Laravel\NativellaServiceProvider`. The application runs with the `nativella`
command: see [Installation](https://lyrixx.github.io/nativella/docs/installation/) and
[Laravel](https://lyrixx.github.io/nativella/docs/laravel/).

## Contributing

This package is developed in [`packages/laravel`](https://github.com/lyrixx/nativella/tree/main/packages/laravel) of the
[Nativella repository](https://github.com/lyrixx/nativella); [lyrixx/nativella-laravel](https://github.com/lyrixx/nativella-laravel)
is a read-only copy of it. Report issues and send pull requests to the Nativella repository.

## License

MIT, see [LICENSE](LICENSE).
