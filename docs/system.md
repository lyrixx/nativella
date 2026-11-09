# System

`Nativella\System` tells what the application runs on, and how the system is set up.

```php
use Nativella\System\Theme;

$info = $system->info();
$isMac = 'macos' === $info->os;

$dark = Theme::Dark === $system->theme();
```

| Method | |
|---|---|
| `info(): SystemInfo` | The operating system, its version, the machine and the locale. |
| `theme(?string $window = null): Theme` | A window's theme, which follows the system theme: `Theme::Light` or `Theme::Dark`. |

## System info

`Nativella\System\SystemInfo`:

| Property | Type | |
|---|---|---|
| `os` | `string` | `linux`, `macos`, `windows`… |
| `family` | `string` | `unix` or `windows`. |
| `arch` | `string` | `x86_64`, `aarch64`… |
| `version` | `string` | The version of the operating system. |
| `hostname` | `?string` | |
| `locale` | `?string` | The user's language, as a BCP 47 tag, like `fr-FR`. |

The locale is a good default for the application's language, before the user picks one.

## Theme

The pages can follow the system theme on their own, with the `prefers-color-scheme` media query. PHP needs `theme()` for what CSS cannot reach, such as a tray icon or an image generated on the server.

When the user switches between light and dark, every window receives `Nativella\Event\WindowThemeChanged`, with the new `theme`. See [Windows](windows.md#window-events).
