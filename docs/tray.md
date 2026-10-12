# Tray

`Nativella\Tray` puts an icon in the system tray (the menu bar on macOS), with a menu. An application has one tray icon at most.

```php
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;

$tray->set(
    tooltip: 'Notes',
    menu: [
        MenuItem::link('tray.new', 'New note', '/notes/new'),
        MenuItem::action('tray.show', 'Show Notes'),
        MenuItem::separator(),
        MenuItem::role(Role::Quit),
    ],
);
```

While the tray icon exists, closing the last window does not quit the application: it keeps running in the tray. Give the user a way out: the `Quit` [role](menus.md#roles), or an item whose click calls `App::quit()`.

## Methods

| Method | |
|---|---|
| `set(?string $tooltip = null, list<MenuItem> $menu = [], ?string $icon = null, ?string $title = null): void` | Creates the tray icon, or updates it. |
| `remove(): void` | Removes the tray icon. Closing the last window quits the application again. |

`set()` replaces everything at once: call it again with new values to change the tooltip, the icon or the menu.

| Argument | |
|---|---|
| `$tooltip` | Shown when the mouse rests on the icon. |
| `$menu` | The [menu items](menus.md#menu-items) of the tray menu. Their clicks dispatch `MenuItemClicked`, like those of the application menu. |
| `$icon` | The absolute path of a PNG file. Defaults to the icon of the application. |
| `$title` | A text next to the icon, on macOS and Linux. |

## Clicks

A click on the icon dispatches `Nativella\Event\TrayClicked`, whose `button` is `left`, `right` or `middle`. Typically, a left click brings the main window back:

```php
use Nativella\Event\TrayClicked;

if ($event instanceof TrayClicked && 'left' === $event->button) {
    // Focuses the main window, or opens it again, as composer.json describes it.
    $window->open('main');
}
```

> [!NOTE]
> Linux reports no click on tray icons: only the menu works there. Put every action of the tray in its menu.

## Example: keep running in the tray

Once the tray icon exists, the user can close the window and the application lives on. The tray menu opens the window again, or quits:

```php
use Nativella\Event\AppBooted;
use Nativella\Event\MenuItemClicked;
use Nativella\Menu\MenuItem;

if ($event instanceof AppBooted) {
    $tray->set(tooltip: 'Timer', menu: [
        MenuItem::action('tray.show', 'Show the timer'),
        MenuItem::separator(),
        MenuItem::action('tray.quit', 'Quit'),
    ]);
}

if ($event instanceof MenuItemClicked) {
    match ($event->id) {
        'tray.show' => $window->open('main'),
        'tray.quit' => $app->quit(),
        default => null,
    };
}
```

Opened again, the main window starts from the [`window`](configuration.md#window) options of `composer.json`. Starting the application a second time brings it back too: see [A second launch](lifecycle.md#a-second-launch).
