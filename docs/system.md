# System

`Nativella\System` tells what the application runs on, and how the system is set up.

```php
use Nativella\System\Theme;

$info = $system->info();
$isMac = 'macos' === $info->platform;

$dark = Theme::Dark === $system->theme();
```

| Method | |
|---|---|
| `info(): SystemInfo` | The operating system, its version, the machine and the locale. |
| `theme(?string $window = null): Theme` | A window's theme, which follows the system theme: `Theme::Light` or `Theme::Dark`. See [Theme](#theme) for an application without a window. |

## System info

`Nativella\System\SystemInfo`:

| Property | Type | |
|---|---|---|
| `platform` | `string` | `linux`, `macos`, `windows`, `android` or `ios`, as [`Runtime::platform()`](configuration.md#what-php-sees). |
| `family` | `string` | `unix` or `windows`. |
| `arch` | `string` | `x86_64`, `aarch64`… |
| `version` | `string` | The version of the operating system. |
| `hostname` | `?string` | |
| `locale` | `?string` | The user's language, as a BCP 47 tag, like `fr-FR`. |

The locale is a good default for the application's language, before the user picks one.

## Theme

The pages can follow the system theme on their own, with the `prefers-color-scheme` media query. PHP needs `theme()` for what CSS cannot reach, such as a tray icon or an image generated on the server.

While the application has no window, when it handles `AppBooted` or lives in the [tray](tray.md), `theme()` without a label answers the desktop's theme on Linux, and `Theme::Light` elsewhere, until a window opens. A label that names no open window throws.

When the user switches between light and dark, every window receives `Nativella\Event\WindowThemeChanged`, with the new `theme`. See [Windows](windows.md#window-events).
