# Screens

`Nativella\Screen` describes the displays connected to the machine, and where the mouse cursor is.

```php
$display = $screen->primary();

$window->setSize(
    $display->workArea->width * 0.6,
    $display->workArea->height * 0.8,
);
$window->center();
```

| Method | |
|---|---|
| `all(): list<Display>` | Every display. |
| `primary(): ?Display` | The primary display, null when the system names none: under Wayland, among others, where `Display::$primary` is always false too. |
| `cursorPosition(): Point` | Where the mouse cursor is. |

## Display

`Nativella\Screen\Display`:

| Property | Type | |
|---|---|---|
| `name` | `?string` | The name the system gives the display. |
| `primary` | `bool` | |
| `bounds` | `Rectangle` | The whole display. |
| `workArea` | `Rectangle` | The display without the taskbar, the dock or the menu bar: where windows go. |
| `scaleFactor` | `float` | Physical pixels per logical pixel: 2.0 on a typical high-density display. |

`Nativella\Screen\Rectangle` has `x`, `y`, `width` and `height`, in logical pixels, like CSS pixels and window sizes. Displays share one desktop: the position of a display is where it sits on it.

## Cursor

`cursorPosition()` returns a `Nativella\Screen\Point`, with `x` and `y` in physical pixels, from the top left corner of the desktop. Divide by the `scaleFactor` of the display to compare it with window positions, which are logical.
