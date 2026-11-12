# App

`Nativella\App` is about the application as a whole: what it is called, where its files go, and when it starts and stops.

```php
use Nativella\App\Directory;

$info = $app->info();             // name, version, identifier, nativellaVersion
$downloads = $app->path(Directory::Downloads);

$app->enableAutostart();
$app->setBadgeCount(3);
```

## Methods

| Method | |
|---|---|
| `info(): AppInfo` | The application's name, version and identifier, from [`composer.json`](configuration.md), and the Nativella version. |
| `path(Directory $directory): string` | The absolute path of a user or application directory. |
| `thirdPartyNotices(): ?string` | The [licenses](building.md#licenses) of the software the application contains (PHP and Nativella's libraries), for an "about" page. Null when the `nativella` command is a development build, which has none. |
| `quit(): void` | Quits, once the current request is answered. |
| `restart(): void` | Restarts, once the current request is answered. |
| `hide(): void` | Hides every window of the application. macOS only. |
| `show(): void` | Shows the windows `hide()` hid. macOS only. |
| `setBadgeCount(?int $count, ?string $window = null): void` | Shows a count on the application icon, in the dock or the taskbar. Null removes it. |
| `enableAutostart(): void` | Starts the application when the user logs in. |
| `disableAutostart(): void` | |
| `isAutostartEnabled(): bool` | |

`hide()` and `show()` only work on macOS. Elsewhere they throw a `NativeCallException`: hide and show the windows themselves, with [`Window`](windows.md).

The badge shows up on macOS, and on the Linux desktops that implement Unity's launcher API. Windows has no badge count. A count of 0 removes the badge, as null does.

On phones, `setBadgeCount()`, `enableAutostart()`, `disableAutostart()` and `path(Directory::Desktop)` throw, and `isAutostartEnabled()` answers false. See [What runs where](platforms.md).

## Directories

`Nativella\App\Directory` names the directories `path()` knows:

| Case | |
|---|---|
| `AppData` | The application's [data directory](configuration.md#the-data-directory), where the database lives. Follows `NATIVELLA_DATA_DIR`. |
| `AppConfig` | Where the application may keep configuration files. |
| `AppCache` | Where it may keep files it can rebuild. |
| `AppLog` | Where it may write its logs. |
| `Temp` | The system temporary directory. |
| `Home`, `Desktop`, `Documents`, `Downloads`, `Music`, `Pictures`, `Videos` | The user's directories. |

The application directories are named after the [identifier](configuration.md#identifier). They may not exist yet, so create them before writing to them. `AppData` is the exception, since Nativella creates it at startup.

```php
use Nativella\App\Directory;

$export = $app->path(Directory::Downloads).'/notes-'.date('Y-m-d').'.zip';
```

## Autostart

`enableAutostart()` registers the application to start when the user logs in: a `.desktop` entry in `~/.config/autostart/` on Linux, a launch agent in `~/Library/LaunchAgents/` on macOS, a registry key on Windows. Offer it as a setting rather than enabling it on your own.

The entry starts the executable that is running now. For an application started with `nativella run`, that is the `nativella` command, not your project, so autostart makes sense for a [standalone build](building.md).
