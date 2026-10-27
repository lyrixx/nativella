# Menus

`Nativella\Menu` sets the menu of the application and opens context menus. Both are built from `Nativella\Menu\MenuItem`, as is the [tray](tray.md) menu.

```php
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;

$menu->set(
    MenuItem::submenu(
        'File',
        MenuItem::link('new', 'New note', '/notes/new', 'CmdOrCtrl+N'),
        MenuItem::action('export', 'Export…', 'CmdOrCtrl+E'),
        MenuItem::separator(),
        MenuItem::role(Role::Quit),
    ),
    MenuItem::editMenu(),
    MenuItem::submenu(
        'View',
        MenuItem::checkbox('sidebar', 'Show sidebar', checked: true, accelerator: 'CmdOrCtrl+B'),
    ),
);
```

Set the menu while handling [`AppBooted`](lifecycle.md#booting), so that it is there when the first window opens. Calling `set()` again replaces it; `update()` changes one item, to disable it, rename it or check it:

```php
$menu->update('export', enabled: $notes->count() > 0);
$menu->update('sidebar', checked: false);
```

On Linux and Windows, each window shows the menu under its title bar. On macOS, the menu bar belongs to the application, its top level only holds submenus, and the first one is named after the application: start with `MenuItem::appMenu('Notes')` there.

## Methods

| Method | |
|---|---|
| `set(MenuItem ...$items): void` | Sets the application menu. On macOS, the top level only holds submenus. |
| `remove(): void` | Removes the application menu. |
| `update(string $id, ?string $label = null, ?bool $enabled = null, ?bool $checked = null): void` | Changes the item `$id`, in the application menu and the [tray](tray.md) menu; what is null stays as it is. Throws when neither has it, or for `$checked` when it is not a checkbox. Context menus are gone once closed: open another. |
| `popup(MenuItem ...$items): void` | Opens a context menu at the mouse cursor, in the window that made the request. |

## Menu items

| Factory | |
|---|---|
| `MenuItem::action(string $id, string $label, ?string $accelerator = null, bool $enabled = true)` | An item that dispatches `MenuItemClicked` with its id. |
| `MenuItem::link(string $id, string $label, string $url, ?string $accelerator = null, bool $enabled = true)` | An action that also opens `$url`, a path of the application, in the focused window. |
| `MenuItem::checkbox(string $id, string $label, bool $checked = false, ?string $accelerator = null, bool $enabled = true)` | An item with a check mark, which the system toggles on click. |
| `MenuItem::separator()` | |
| `MenuItem::submenu(string $label, MenuItem ...$items)` | |
| `MenuItem::role(Role $role)` | An entry the system implements: copy, paste, quit… See [Roles](#roles). |
| `MenuItem::appMenu(string $name)` | The application submenu of macOS: about, services, hide, quit. Leave it out elsewhere. |
| `MenuItem::editMenu(string $label = 'Edit')` | Undo, redo, cut, copy, paste, select all. |
| `MenuItem::windowMenu(string $label = 'Window')` | Minimize, maximize, full screen, close. |

Ids are yours to choose. They come back in `MenuItemClicked::$id`, so keep them unique across the application menu, context menus and the tray menu.

### Roles

`Nativella\Menu\Role` lists the entries the system implements: `About`, `CloseWindow`, `Copy`, `Cut`, `Fullscreen`, `Hide`, `HideOthers`, `Maximize`, `Minimize`, `Paste`, `Quit`, `Redo`, `SelectAll`, `Services`, `ShowAll` and `Undo`. They come with the label and the shortcut the platform expects, and their clicks do not reach PHP: the system, or Nativella, handles them.

| Role | macOS | Linux |
|---|---|---|
| `Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll` | The system | Nativella, on the focused page. The page keeps its own keyboard shortcuts. |
| `Minimize`, `Maximize`, `Fullscreen`, `CloseWindow` | The system | Nativella, on the focused window. `Maximize` and `Fullscreen` (F11) toggle. |
| `Quit` | The system | Nativella (`CmdOrCtrl+Q`) |
| `About` | The system | The system, with the name, version and icon of the application |
| `Hide`, `HideOthers`, `ShowAll`, `Services` | The system | Left out |

GTK, the toolkit of Linux, implements few of these entries, so Nativella does it itself there. Windows is untested; the menu library Tauri uses does not implement `Fullscreen`, `HideOthers`, `ShowAll` and `Services` on Windows, and shows them disabled.

On macOS, keep an Edit menu holding `Copy` and `Paste` (`MenuItem::editMenu()`): without it, the usual keyboard shortcuts may not reach text fields.

### Accelerators

An accelerator is the keyboard shortcut of an item, shown next to its label: modifiers and a key, joined by `+`, like `CmdOrCtrl+Shift+N`. `CmdOrCtrl` is Command on macOS and Control elsewhere. The other modifiers are `Shift`, `Control`, `Alt` and `Super`. An accelerator only works while the application has the focus; for shortcuts that work in the background, see [Global shortcuts](global-shortcuts.md).

## Clicks

A click on an action or a checkbox dispatches `Nativella\Event\MenuItemClicked`:

```php
use Nativella\Event\MenuItemClicked;

if ($event instanceof MenuItemClicked) {
    match ($event->id) {
        'export' => $exporter->exportAll(),
        'sidebar' => $settings->set('sidebar', $event->checked),
        default => null,
    };
}
```

`$event->checked` is the new state of a checkbox, and null for the other items. A link item opens its URL in the focused window, then dispatches the event like any action.

The click reaches the pages too, as a `nativella:menu.itemClicked` DOM event whose `detail` is `{id, checked}`: a page can react without a round trip to PHP. See [Pages and JavaScript](javascript.md).

## Context menus

`popup()` opens a menu at the mouse cursor. It needs a request from the page, so the page asks for it on right click:

```js
document.addEventListener('contextmenu', (event) => {
    const note = event.target.closest('[data-note]');
    if (!note) {
        return;
    }

    event.preventDefault();
    fetch(`/notes/${note.dataset.note}/context-menu`, { method: 'POST' });
});
```

```php
use Nativella\Menu\MenuItem;

// POST /notes/{id}/context-menu
$menu->popup(
    MenuItem::action('note.pin.'.$id, 'Pin'),
    MenuItem::action('note.duplicate.'.$id, 'Duplicate'),
    MenuItem::separator(),
    MenuItem::action('note.delete.'.$id, 'Delete…'),
);
```

The menu opens in the window that sent the request. The clicked item comes back as a `MenuItemClicked` event, so encode in its id what it acts on.
