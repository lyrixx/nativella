# Windows

`Nativella\Window` opens and drives the application's windows. Each window has a label, `main` for the one Nativella opens at startup, and loads the application's pages like a browser tab would.

```php
use Nativella\Window\WindowOptions;

$window->open('settings', new WindowOptions(
    url: '/settings',
    title: 'Settings',
    width: 480,
    height: 600,
    resizable: false,
    center: true,
));
```

With a framework integration, inject `Nativella\Window`. In plain PHP, use `Nativella::create()->window`.

## Which window

Every method but `open()` and `all()` takes an optional window label, last. Without it, the method acts on:

1. the window whose page made the current request;
2. during a [native event](lifecycle.md#native-events), which no window made, the focused window;
3. failing that, the `main` window, then any window.

So `$window->setTitle('Draft')` in a controller retitles the window the user is looking at, and `$window->setTitle('Draft', 'settings')` the settings window. A label that matches no open window throws a `Nativella\Exception\NativeCallException`.

Labels are made of letters, digits, `-`, `/`, `:` and `_`.

## Opening a window

`open(string $window, ?WindowOptions $options = null): WindowInfo` opens the window labelled `$window` and returns its [`WindowInfo`](#window-info). If a window with that label is already open, `open()` focuses it instead, and ignores the options.

The `main` window starts from the [`window`](configuration.md#window) options in `composer.json`, and the options you pass complete them. `$window->open('main')` opens it again with them, [where the user left it](#what-a-window-remembers).

The new window's first request waits for the current one to finish, since one PHP thread serves them all.

## Window options

`Nativella\Window\WindowOptions` takes named arguments. Anything left null keeps the system default. The same keys, in a JSON object, configure the main window in [`composer.json`](configuration.md#window).

| Option | Type | Default | |
|---|---|---|---|
| `url` | `?string` | `'/'` | An application path (`/settings`). The window does not load another website: see [What a window loads](#what-a-window-loads). |
| `title` | `?string` | The application name | |
| `width`, `height` | `?float` | 1024 × 768 | The size of the page area, in logical pixels. |
| `minWidth`, `minHeight` | `?float` | | Applied when both are set. |
| `maxWidth`, `maxHeight` | `?float` | | Applied when both are set. |
| `x`, `y` | `?float` | | The position on the screen, applied when both are set. |
| `center` | `?bool` | `false` | Centers the window on the screen. |
| `resizable` | `?bool` | `true` | |
| `maximizable`, `minimizable`, `closable` | `?bool` | `true` | |
| `maximized`, `fullscreen` | `?bool` | `false` | How the window starts. |
| `decorations` | `?bool` | `true` | `false` for a frameless window, without a title bar. |
| `alwaysOnTop` | `?bool` | `false` | |
| `focused` | `?bool` | `true` | Whether the window takes the focus when it opens. |
| `visible` | `?bool` | `true` | `false` opens it hidden, until `show()`. |
| `skipTaskbar` | `?bool` | `false` | Leaves the window out of the taskbar. |

### What a window remembers

On computers, a window opens again where the user left it. For each label, Nativella keeps the window's size, its position, whether it was maximized or in full screen, and whether it had decorations. It applies them when a window with that label opens again, in the same run or at the next start. They win over `width`, `height`, `x`, `y`, `center`, `maximized`, `fullscreen` and `decorations`, in `WindowOptions` and in `composer.json`, so these options only count the first time a label opens. The position is restored only when its display is still there.

The state is written when the application quits, to `.window-state.json` in the application's configuration directory, [`Directory::AppConfig`](app.md#directories): `~/.config/<identifier>` on Linux. Delete the file to open the windows from their options again.

## Driving windows

| Method | |
|---|---|
| `close(?string $window = null): void` | Closes the window, once its page [lets it go](javascript.md#unsaved-changes). Closing the last one quits the application, unless it has a [tray](tray.md). |
| `all(): list<WindowInfo>` | Every open window, sorted by label. |
| `get(?string $window = null): WindowInfo` | |
| `setTitle(string $title, ?string $window = null): void` | |
| `setSize(float $width, float $height, ?string $window = null): void` | In logical pixels. |
| `setPosition(float $x, float $y, ?string $window = null): void` | In logical pixels, from the top left corner of the screen. |
| `center(?string $window = null): void` | |
| `setResizable(bool $enabled, ?string $window = null): void` | |
| `setAlwaysOnTop(bool $enabled, ?string $window = null): void` | |
| `setFullscreen(bool $enabled, ?string $window = null): void` | |
| `minimize(?string $window = null): void` | |
| `maximize(?string $window = null): void` | |
| `unmaximize(?string $window = null): void` | |
| `show(?string $window = null): void` | |
| `hide(?string $window = null): void` | A hidden window is still open, and `show()` brings it back. |
| `focus(?string $window = null): void` | |
| `reload(?string $window = null): void` | Reloads the page. |
| `setProgress(?int $percent, ?string $window = null): void` | Shows a progress bar, from 0 to 100, on the window's icon in the taskbar or the dock (Windows, macOS, GNOME with libunity). Null removes it. Desktop only. |
| `print(?string $window = null): void` | Opens the system print dialog, which can also print to PDF, for the window's page. |
| `printToPdf(string $path, ?string $window = null): void` | Writes the window's page to a PDF file, at an absolute path, without a dialog, for an invoice or a report. `@media print` styles apply. Linux only, for now. |
| `navigate(string $url, ?string $window = null): void` | Loads an application path (`/notes/42`). The URL of another website opens in the system browser, as a link to it does. |
| `openDevtools(?string $window = null): void` | Opens the window's web inspector. Debug builds of Nativella only. Elsewhere it throws. |

On phones, a window fills the screen, so `center()`, `minimize()`, `maximize()`, `unmaximize()`, `setAlwaysOnTop()`, `setFullscreen()`, `setProgress()`, `print()` and `printToPdf()` throw a `NativeCallException` there, and the size, position and decoration options are ignored. See [What runs where](platforms.md).

## Window info

`Nativella\Window\WindowInfo` describes a window as it is at the time of the call:

| Property | Type | |
|---|---|---|
| `window` | `string` | Its label, the value that the `$window` argument of methods and events takes. |
| `title` | `?string` | |
| `url` | `?string` | The page URL, like `php://localhost/notes/42`. |
| `width`, `height` | `?float` | The size of the page area, in logical pixels. |
| `x`, `y` | `?float` | The position of the window, in logical pixels. |
| `focused`, `maximized`, `minimized`, `fullscreen`, `visible` | `bool` | |

The size and position are null when the system does not report them.

## Window events

| Event | |
|---|---|
| `WindowFocused` | The window got the focus. |
| `WindowBlurred` | The window lost the focus. |
| `WindowClosed` | The window was closed. |
| `WindowThemeChanged` | The system switched between light and dark: `$event->theme`. |
| `WindowDownloaded` | A [download](downloads.md) of the window was saved: `$event->path`, absolute. |
| `WindowDragEntered` | Files dragged from the system entered the window: `$event->paths`. |
| `WindowDragLeft` | They left it without being dropped. |
| `WindowFilesDropped` | Files were dropped on the window: `$event->paths`, absolute. |

Each one has the window's label in `$event->window`. See [Lifecycle and events](lifecycle.md) for how to listen to them.

## Example: a preferences window

A menu item opens a second window, which closes itself once the form is saved:

```php
use Nativella\Event\MenuItemClicked;
use Nativella\Window\WindowOptions;

// When the menu item "preferences" is clicked:
if ($event instanceof MenuItemClicked && 'preferences' === $event->id) {
    $window->open('preferences', new WindowOptions(
        url: '/preferences',
        title: 'Preferences',
        width: 420,
        height: 520,
        resizable: false,
        center: true,
    ));
}

// In the controller that saves the form, called by the preferences window:
$settings->set('theme', $theme);
$window->close();
```

`$window->close()` needs no label here. The request came from the preferences window, so that is the one that closes.

## Frameless windows

A window opened with `decorations: false` has no title bar to move it by. Mark the elements that should act as one with `data-tauri-drag-region`. Dragging them moves the window, and double-clicking them maximizes it.

```html
<header class="title-bar" data-tauri-drag-region>
    <h1 data-tauri-drag-region>Notes</h1>
    <button onclick="…">Close</button>
</header>
```

The attribute applies to the element itself, not to its children, so repeat it on the ones that should drag too, and leave it off buttons.

## What a window loads

A window shows the application's pages. A link to another website opens in the system browser rather than in the window, and so do `mailto:` and `tel:` links. See [Pages and JavaScript](javascript.md#links-and-absolute-urls).
