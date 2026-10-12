# Configuration

A Nativella application describes itself in the `extra.nativella` section of its `composer.json`. Every key is optional: a project without `composer.json` runs too, named after its directory.

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

The identifier names the directories of the application, the [data directory](#the-data-directory) first. Choose it once: changing it moves the application to a new, empty data directory, and its users lose their data.

Defaults to `dev.nativella.` followed by the package name: `acme/note-taker` becomes `dev.nativella.note-taker`.

## `version`

The version of the application, as [`App::info()`](app.md) reports it. Defaults to the `version` of `composer.json`, then to `0.1.0`.

## `icon`

A PNG file, relative to the project. It is the icon of the windows, and of the [tray](tray.md) unless the tray sets its own. Without it, the application uses the Nativella icon.

## `window`

The options of the main window: the one Nativella opens when PHP opens none while handling [`AppBooted`](lifecycle.md#booting). The keys are those of [`WindowOptions`](windows.md#window-options), in camelCase: `url`, `title`, `width`, `height`, `minWidth`, `resizable`, `decorations`…

```json
"window": { "url": "/inbox", "width": 1200, "height": 800, "center": true }
```

Without `window`, the main window opens on `/`, 1024 by 768 pixels, titled after the application.

## `deepLinks`

URL schemes that open the application: with `"deepLinks": ["notes"]`, a `notes://42` link in a browser, a mail or a terminal (`xdg-open notes://42`) starts the application, or wakes it up if it runs, and PHP gets the URL as an [`AppUrlsOpened`](lifecycle.md#native-events) event:

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

Schemes are lowercase letters, digits, `+`, `-` and `.`, starting with a letter; web schemes (`http`, `https`…) are refused. On Linux and Windows, a [standalone executable](building.md) registers its schemes with the system when it starts; `nativella run` does not, as it is not what the system should launch. macOS registers schemes when an application is installed, which Nativella does not do yet. Not on mobile yet.

## `permissions`

The [phone APIs](phone-apis.md) the application uses: `biometrics`, `geolocation`, `haptics`. Mobile builds include these, and only these, with the permissions they declare to Android. Computers ignore it.

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
| `.nativella-secret` | The secret of the installation: [`NATIVELLA_SECRET`](#what-php-sees). |
| `.nativella-cookies` | The cookies with an expiry date, which outlive the application as in a browser. Session cookies end with it. |

## Environment variables

| Variable | Effect |
|---|---|
| `NATIVELLA_DATA_DIR` | Moves the data directory: handy to start from a clean state while developing, or to run two copies side by side. |
| `NO_AT_BRIDGE` | Nativella sets it to `1` on Linux unless it is already set, to work around a crash of WebKitGTK in the accessibility bridge. See [Limitations](limitations.md#linux). |

Like PHP-FPM without `clear_env`, PHP gets the environment of the process in `$_SERVER`: `APP_ENV=prod nativella run .` works as you would expect.

## What PHP sees

On top of the usual CGI variables, the runtime sets these in `$_SERVER`. A request from a page cannot forge them: its headers only ever become `HTTP_*` variables.

| Variable | Value |
|---|---|
| `NATIVELLA_DATA_DIR` | The [data directory](#the-data-directory). |
| `NATIVELLA_SECRET` | A random secret, unique to the installation: 64 hexadecimal characters, created on the first launch and kept in the data directory. Sign and encrypt with it, rather than with a secret of `.env` that every user would share. |
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
| `Runtime::isNative(): bool` | Whether PHP runs in Nativella: `nativella_call()` exists. |
| `Runtime::dataDir(): ?string` | The data directory. |
| `Runtime::secret(): ?string` | The secret of the installation. |
| `Runtime::platform(): ?string` | The platform. |
| `Runtime::isMobile(): bool` | Whether the platform is `android` or `ios`. |
| `Runtime::window(): ?string` | The label of the window of the request. |
| `Runtime::isInternalRequest(): bool` | Whether the runtime sent the request. |
| `Runtime::version(): ?string` | The version of Nativella. |

Each one returns null, or false, outside Nativella.

## `php.ini`

The embedded PHP reads the configuration of the embed SAPI of the system: `/etc/php/8.5/embed/php.ini` and `/etc/php/8.5/embed/conf.d/` on Ubuntu. Settings that matter for a web server matter here too: `memory_limit`, `opcache.enable`, `display_errors`…

Nativella overrides two of them, which point at the system otherwise:

| Setting | Value |
|---|---|
| `date.timezone` | The time zone of the user, so that dates are local. |
| `session.save_path` | `sessions/` in the cache directory of the application. |

`max_execution_time` applies as usual, except that a native call that waited more than a second for the user starts its count again: a user may leave a [dialog](dialogs.md) open as long as they like, as `sleep()` does not count on Linux either. Quick native calls do not: a loop of them still times out.
