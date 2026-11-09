# Configuration

A Nativella application describes itself in the `extra.nativella` section of its `composer.json`. Every key is optional, and a project without `composer.json` runs too, named after its directory.

```json title="composer.json"
{
    "name": "acme/notes",
    "extra": {
        "nativella": {
            "name": "Notes",
            "identifier": "com.acme.notes",
            "version": "1.2.0",
            "icon": "assets/icon.png",
            "window": {
                "title": "Notes",
                "width": 1000,
                "height": 700,
                "minWidth": 640,
                "minHeight": 420
            },
            "deepLinks": ["notes"],
            "permissions": ["haptics"]
        }
    }
}
```

An unknown key, or a value of the wrong type, stops the application at startup with a message that names it.

## `name`

The name of the product: the title of windows that set none, the "About" menu entry, the autostart entry, and the file name of the [standalone executable](building.md).

Defaults to the package name of `composer.json`, title-cased: `acme/note-taker` becomes "Note Taker". Without a package name, the directory name is used the same way.

## `identifier`

A reverse domain name, unique to the application: `com.acme.notes`. Letters, digits and dashes, separated by dots.

The identifier names the application's directories, the [data directory](#the-data-directory) first. Choose it once, because changing it moves the application to a new, empty data directory, and its users lose their data.

Defaults to `dev.nativella.` followed by the package name: `acme/note-taker` becomes `dev.nativella.note-taker`.

On Android, the application ID, what Android knows the application by, is the identifier with its dashes as underscores, as Android accepts no dash: `dev.nativella.note_taker`. It needs at least two segments, each starting with a letter, so `com.acme.2048` cannot make one. See [Mobile](mobile.md#building-an-apk).

On iOS, the bundle ID is the identifier as it is, because Apple accepts letters, digits, dashes and dots there. The provisioning profile of an app for devices must be for it, or for a wildcard App ID that matches it, such as `com.acme.*`. See [Mobile](mobile.md#building-an-ios-app).

## `version`

The application's version, as [`App::info()`](app.md) reports it. Defaults to the `version` of `composer.json`, then to `0.1.0`. It is a semantic version, `1.2.0`, which [updates](updates.md) compare; `1.2` and `1` stand for `1.2.0` and `1.0.0`. Nativella refuses to start with anything else, like `v1.2.0` or `dev-main`.

## `icon`

A PNG file, relative to the project. It is the icon of the windows, of the [tray](tray.md) unless the tray sets its own, and the launcher icon of the [Android application](mobile.md#building-an-apk). Without it, the application uses the Nativella icon.

The windows of a [standalone executable](building.md) take it from the project the executable carries, so the build refuses an icon that does not [ship](building.md#what-ships), such as one under `tests/`. An icon that cannot be read leaves the windows with the Nativella icon, and a warning.

Make it square, 512 or 1024 pixels. Android shows it at up to 192 pixels, and `nativella android` warns below that. A rectangle is centered on a transparent square.

## `window`

The options of the main window, the one Nativella opens when PHP opens none while handling [`AppBooted`](lifecycle.md#booting). The keys are those of [`WindowOptions`](windows.md#window-options), in camelCase: `url`, `title`, `width`, `height`, `minWidth`, `resizable`, `decorations`…

```json
"window": { "url": "/inbox", "width": 1200, "height": 800, "center": true }
```

Without `window`, the main window opens on `/`, 1024 by 768 pixels, titled after the application.

## `deepLinks`

URL schemes that open the application. With `"deepLinks": ["notes"]`, a `notes://42` link in a browser, a mail or a terminal (`xdg-open notes://42`) starts the application, or wakes it up if it runs, and PHP gets the URL as an [`AppUrlsOpened`](lifecycle.md#native-events) event:

```php
use Nativella\Event\AppUrlsOpened;

#[AsEventListener]
public function __invoke(AppUrlsOpened $event): void
{
    foreach ($event->urls as $url) {
        // "notes://42": 42 is the host of the URL.
        $this->window->navigate('/notes/'.parse_url($url, PHP_URL_HOST));
    }
}
```

Schemes are lowercase letters, digits, `+`, `-` and `.`, starting with a letter; web schemes (`http`, `https`…) are refused. On Linux and Windows, a [standalone executable](building.md) registers its schemes with the system when it starts; `nativella run` does not, as it is not what the system should launch. macOS registers schemes when an application is installed, which Nativella does not do yet. Deep links are not available on mobile yet.

## `permissions`

The [phone APIs](phone-apis.md) the application uses (`barcodeScanner`, `biometrics`, `geolocation`, `haptics`, `nfc`), and what its pages may use of the device (`camera`, `microphone`, and the location, with `geolocation` too). Mobile applications have the phone APIs it lists and only those, on Android with the permissions they declare. Computers ignore the phone APIs, but not what the pages may use.

## `updates`

Where a standalone executable finds its updates: the `endpoint` of a document that describes the latest release, and the `publicKey` its executables are signed with. See [Updates](updates.md).

## The data directory

Each application gets a directory for its data, unique to the application and the user, named after its identifier:

| Platform | Data directory |
|---|---|
| Linux | `~/.local/share/<identifier>` (or `$XDG_DATA_HOME/<identifier>`) |
| macOS | `~/Library/Application Support/<identifier>` |
| Windows | `%APPDATA%\<identifier>` |

Nativella creates it at startup and prints it on the standard error output. PHP finds it in `$_SERVER['NATIVELLA_DATA_DIR']`, or with `Nativella\Runtime::dataDir()`. The [database](database.md) and the [settings](settings.md) live there, and so do two files of Nativella's own:

| File | |
|---|---|
| `.nativella-secret` | The installation's secret, [`NATIVELLA_SECRET`](#what-php-sees). Back it up with the database, because what the framework encrypted with it is unreadable without it. A file that holds no secret, after a restore gone wrong, is set aside as `.nativella-secret.corrupted-<time>`, never overwritten, and a new secret replaces it. |
| `.nativella-cookies` | The cookies with an expiry date, which outlive the application as in a browser. Session cookies end with it. |

## Environment variables

| Variable | Effect |
|---|---|
| `NATIVELLA_DATA_DIR` | Moves the data directory, which is handy to start from a clean state while developing, or to run two copies side by side. |
| `NATIVELLA_RELOAD` | `0` stops `nativella run` from [reloading the windows](getting-started.md#reloading) when a file of the project changes. |
| `NO_AT_BRIDGE` | Nativella sets it to `1` on Linux unless it is already set, to work around a WebKitGTK crash in the accessibility bridge. It sets it for itself, not for the [child processes](child-processes.md) and the browser it starts. See [Limitations](limitations.md#linux). |

Like PHP-FPM without `clear_env`, PHP gets the process environment in `$_SERVER`, so `APP_ENV=prod nativella run .` works as you would expect.

## What PHP sees

On top of the usual CGI variables, the runtime sets these in `$_SERVER`. A request from a page cannot forge them, because its headers only ever become `HTTP_*` variables.

| Variable | Value |
|---|---|
| `NATIVELLA_DATA_DIR` | The [data directory](#the-data-directory). |
| `NATIVELLA_SECRET` | A random secret, unique to the installation, of 64 hexadecimal characters, created on the first launch and kept in the data directory. Sign and encrypt with it, rather than with a secret in `.env` that every user would share. |
| `NATIVELLA_PLATFORM` | `linux`, `macos`, `windows`, `android` or `ios`. |
| `NATIVELLA_VERSION` | The version of Nativella. |
| `NATIVELLA_WINDOW` | The label of the window whose page made the request. Absent for internal requests. |
| `NATIVELLA_INTERNAL` | `1` when the runtime itself sent the request, to deliver a [native event](lifecycle.md#native-events). |
| `SERVER_SOFTWARE` | `nativella` |
| `REMOTE_ADDR` | `127.0.0.1` |
| `HTTP_HOST` | `localhost` |

`Nativella\Runtime` reads them for you:

| Method | |
|---|---|
| `Runtime::isNative(): bool` | Whether PHP runs in Nativella, meaning `nativella_call()` exists. False under the [fake client](testing.md) of the tests, so guard native features with `Nativella::isNative()`, which asks the client. |
| `Runtime::dataDir(): ?string` | The data directory. |
| `Runtime::secret(): ?string` | The installation's secret. |
| `Runtime::platform(): ?string` | The platform. |
| `Runtime::isMobile(): bool` | Whether the platform is `android` or `ios`. |
| `Runtime::window(): ?string` | The label of the request's window. |
| `Runtime::isInternalRequest(): bool` | Whether the runtime sent the request. |
| `Runtime::version(): ?string` | The version of Nativella. |

Each one returns null, or false, outside Nativella.

## `php.ini`

PHP reads Nativella's php.ini, [`crates/runtime/php.ini`](../crates/runtime/php.ini), compiled into the `nativella` command, and no other. It ignores a php.ini on the machine, which belongs to another PHP, and the files `PHPRC` and `PHP_INI_SCAN_DIR` point to. It keeps PHP's defaults, except for these:

| Setting | Value |
|---|---|
| `display_errors` | `Off`: errors go to the standard error output, the terminal of `nativella run` (logcat on Android), not into the page. A fatal error answers a 500, which PHP only does when it does not display the error. |
| `zend.assertions` | `-1`, as in production. |
| `post_max_size`, `upload_max_filesize` | `512M`, because PHP's default 2 MB is less than a phone photo. |
| `hard_timeout` | `0`, see below. |

An application changes the settings it needs with `ini_set()`, at the top of its front controller: `memory_limit`, `max_execution_time`… `ini_set()` cannot change the ones PHP reads before the script runs, like `post_max_size` and `upload_max_filesize`, nor the ones it reads only when it starts, like opcache's memory size. Those take a change to that file, in a `nativella` [of your own](custom-php.md).

At startup, Nativella sets two more, which point at the system otherwise:

| Setting | Value |
|---|---|
| `date.timezone` | The user's time zone, so that dates are local. |
| `session.save_path` | `sessions/` in the application's cache directory. |

`max_execution_time` applies as usual, except that a native call that waited more than a second for the user starts its count again, so a user may leave a [dialog](dialogs.md) open as long as they like, as `sleep()` does not count on Linux either. Quick native calls do not restart it, so a loop of them still times out. The hard timeout is off, `hard_timeout = 0`, because PHP counts the time of the whole process and would kill it, and the application with it, two seconds after the limit.

A `nativella` linked with [the system PHP](development.md#the-php-of-the-system), in development, first reads that PHP's php.ini, which loads its extensions: `/etc/php/8.5/embed/php.ini` and `/etc/php/8.5/embed/conf.d/` on Ubuntu. Nativella's applies over it.
