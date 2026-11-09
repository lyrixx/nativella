# Nativella

**[Website](https://lyrixx.github.io/nativella/)** · **[Documentation](https://lyrixx.github.io/nativella/docs/)** ·
**[Getting started](https://lyrixx.github.io/nativella/docs/getting-started/)** ·
**[Download](https://github.com/lyrixx/nativella/releases/latest)**

Run a PHP web application as a desktop or mobile application. Your Symfony,
Laravel or plain PHP project opens in a native window. PHP runs inside the
application process and answers the window directly, so no web server is
involved and no port is opened. The window is the system webview, through
Tauri, so the application ships neither Electron nor Node.

Nativella started as a proof of concept of PHP, Rust and Tauri in one process,
to find out if a Tauri window could hand its requests to PHP through a SAPI
written in Rust. The [proof of concept](https://github.com/lyrixx/nativella/tree/850aa2df4b9f15c680b81ca07696914744565496)
is the first commit of this repository, with its measurements and what it ran
into, such as WebKitGTK handling neither cookies nor redirects on a custom
scheme. The rest of the project was built on top of it.

```
system webview ──php://localhost/…──▶ Tauri scheme handler (Rust)
                                        │ public/ file? ─▶ served from disk
                                        ▼
                                      PHP thread ── cookie jar, redirects, URL rewriting
                                        │
                                        ▼
                                      custom SAPI (ext-php-rs) ─▶ public/index.php
                                        │                             │
                    native events ─────▶│◀──── nativella_call() ──────┘
                                        ▼
                         windows, menus, tray, dialogs, clipboard, notifications…
```

- It runs any PHP application with a `public/index.php`. [Symfony](https://lyrixx.github.io/nativella/docs/symfony/) and
  [Laravel](https://lyrixx.github.io/nativella/docs/laravel/) have integrations, and
  [plain PHP](https://lyrixx.github.io/nativella/docs/plain-php/) works too.
- PHP gets [native APIs](https://lyrixx.github.io/nativella/docs/#native-apis) for windows, application and context menus,
  the tray icon, dialogs, notifications, the clipboard, global shortcuts,
  settings, secrets, timers, child processes, power, screens and the shell,
  and receives typed native events (`MenuItemClicked`, `WindowFilesDropped`…).
  Phones add haptics, geolocation, biometrics, a barcode scanner, NFC tags and
  the share sheet.
- The [state](https://lyrixx.github.io/nativella/docs/database/) lives in an SQLite database in the application's
  data directory, migrated when the application starts (Doctrine, Eloquent).
- To [distribute it](https://lyrixx.github.io/nativella/docs/building/), `nativella build` writes one executable
  that carries the project and PHP, so the machine does not need PHP.
- The same project builds into an [Android APK and an iOS app](https://lyrixx.github.io/nativella/docs/mobile/),
  experimentally, without Android's or Apple's tools.

Nativella is young and experimental, and is developed on Linux. Read
[Limitations and roadmap](https://lyrixx.github.io/nativella/docs/limitations/) first.

## Try it

[Download `nativella`](https://lyrixx.github.io/nativella/docs/installation/#download), for Linux x86_64 or
macOS arm64, then, from a clone of the repository:

```bash
(cd demos/symfony && composer install)
nativella run demos/symfony
```

Or build it from source, in Docker, with [Castor](https://castor.jolicode.com),
as the releases are:

```bash
castor static-php:build          # PHP 8.5 as a static library, once
castor nativella:build           # dist/nativella, with PHP inside, and the demos
```

`castor start`, then `castor qa`, runs the checks. See
[Developing Nativella](https://lyrixx.github.io/nativella/docs/development/).

To write your own application, [Getting started](https://lyrixx.github.io/nativella/docs/getting-started/)
goes from an empty directory to a desktop application.

## Repository

| Path | What |
|---|---|
| [`crates/runtime`](crates/runtime) | The runtime: the SAPI, the PHP thread, static files, cookies, redirects, and `nativella_call()`. Framework and GUI agnostic. |
| [`crates/nativella`](crates/nativella) | The shell and the `nativella` command: Tauri windows, the native methods (`src/native/`), events, standalone and mobile bundles. |
| [`packages/nativella`](packages/nativella) | `nativella/nativella`: the PHP API (`Nativella\Window`, `Dialog`, `Menu`…), the events, the fake client for tests. |
| [`packages/symfony`](packages/symfony) | `nativella/symfony-bundle`: services, Symfony events, migrations at boot, profiler panel. |
| [`packages/laravel`](packages/laravel) | `nativella/laravel`: service provider, facades, Laravel events, migrations at boot. |
| [`demos/symfony`](demos/symfony) | Nativella Notes: Symfony 8, Doctrine, menus, tray, context menus, drag and drop. Runs on Android and iOS too. |
| [`demos/laravel`](demos/laravel) | Nativella Tasks: Laravel 13, Eloquent. |
| [`demos/vanilla`](demos/vanilla) | Nativella Snippets: no framework, Doctrine DBAL. |
| [`mobile`](mobile) | What the Android and iOS builds need besides their Castor tasks: the stand-ins that describe PHP for the phone to ext-php-rs. |
| [`website`](website) | The [public website](https://lyrixx.github.io/nativella/): the landing page, generated by PHP, and the documentation of [`docs`](docs), built by Zensical. |

## License

MIT, see [LICENSE](LICENSE).
