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
| `theme(?string $window = null): Theme` | The theme of a window, which follows the one of the system: `Theme::Light` or `Theme::Dark`. |

## System info

`Nativella\System\SystemInfo`:

| Property | Type | |
|---|---|---|
| `os` | `string` | `linux`, `macos`, `windows`… |
| `family` | `string` | `unix` or `windows`. |
| `arch` | `string` | `x86_64`, `aarch64`… |
| `version` | `string` | The version of the operating system. |
| `hostname` | `?string` | |
| `locale` | `?string` | The language of the user, as a BCP 47 tag: `fr-FR`. |

The locale is a good default for the language of the application, before the user picks one.

## Theme

The pages can follow the theme of the system on their own, with the `prefers-color-scheme` media query. PHP needs `theme()` for what CSS cannot reach: an icon of the tray, an image generated on the server.

When the user switches between light and dark, every window receives `Nativella\Event\WindowThemeChanged`, with the new `theme`. See [Windows](windows.md#window-events).
