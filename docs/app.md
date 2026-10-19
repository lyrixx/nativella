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
| `info(): AppInfo` | The name, version and identifier of the application, from [`composer.json`](configuration.md), and the version of Nativella. |
| `path(Directory $directory): string` | The absolute path of a directory of the user or of the application. |
| `thirdPartyNotices(): ?string` | The [licenses](building.md#licenses) of the software the application contains, PHP and the libraries of Nativella, for an "about" page. Null when the `nativella` command is a development build, which has none. |
| `quit(): void` | Quits, once the current request is answered. |
| `restart(): void` | Restarts, once the current request is answered. |
| `hide(): void` | Hides every window of the application. macOS only. |
| `show(): void` | Shows the windows `hide()` hid. macOS only. |
| `setBadgeCount(?int $count, ?string $window = null): void` | Shows a count on the icon of the application, in the dock or the taskbar. Null removes it. |
| `enableAutostart(): void` | Starts the application when the user logs in. |
| `disableAutostart(): void` | |
| `isAutostartEnabled(): bool` | |

`hide()` and `show()` throw a `NativeCallException` on Linux and Windows: hide and show the windows themselves there, with [`Window`](windows.md).

The badge shows up on macOS, and on the Linux desktops that implement the launcher API of Unity. Windows has no badge count. A count of 0 removes the badge, as null does.

## Directories

`Nativella\App\Directory` names the directories `path()` knows:

| Case | |
|---|---|
| `AppData` | The [data directory](configuration.md#the-data-directory) of the application, where the database lives. Follows `NATIVELLA_DATA_DIR`. |
| `AppConfig` | Where the application may keep configuration files. |
| `AppCache` | Where it may keep files it can rebuild. |
| `AppLog` | Where it may write its logs. |
| `Temp` | The temporary directory of the system. |
| `Home`, `Desktop`, `Documents`, `Downloads`, `Music`, `Pictures`, `Videos` | The directories of the user. |

The application directories are named after the [identifier](configuration.md#identifier). They may not exist yet: create them before writing to them. `AppData` is the exception, since Nativella creates it at startup.

```php
use Nativella\App\Directory;

$export = $app->path(Directory::Downloads).'/notes-'.date('Y-m-d').'.zip';
```

## Autostart

`enableAutostart()` registers the application to start when the user logs in: a `.desktop` entry in `~/.config/autostart/` on Linux, a launch agent in `~/Library/LaunchAgents/` on macOS, a registry key on Windows. Offer it as a setting rather than enabling it on your own.

The entry starts the executable that is running now. For an application started with `nativella run`, that is the `nativella` command, not your project: autostart makes sense for a [standalone build](building.md).
