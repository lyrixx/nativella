# Global shortcuts

`Nativella\GlobalShortcut` registers keyboard shortcuts that work even when the application is in the background, or has no window at all.

```php
$globalShortcut->register('CmdOrCtrl+Shift+Space');
```

A press dispatches `Nativella\Event\GlobalShortcutPressed`, whose `accelerator` is the shortcut, as you registered it:

```php
use Nativella\Event\GlobalShortcutPressed;
use Nativella\Window\WindowOptions;

if ($event instanceof GlobalShortcutPressed && 'CmdOrCtrl+Shift+Space' === $event->accelerator) {
    $window->open('quick-note', new WindowOptions(url: '/notes/quick', width: 420, height: 160, alwaysOnTop: true, center: true));
}
```

The press reaches the pages too, as a `nativella:globalShortcut.pressed` DOM event.

## Methods

| Method | |
|---|---|
| `register(string $accelerator): void` | Starts listening. Registering a shortcut twice is harmless. |
| `unregister(string $accelerator): void` | Stops listening. |
| `unregisterAll(): void` | Stops listening to every shortcut of the application. |
| `isRegistered(string $accelerator): bool` | |

Register shortcuts while handling [`AppBooted`](lifecycle.md#booting): registrations do not survive a restart of the application.

## Accelerators

An accelerator is modifiers and a key, joined by `+`: `CmdOrCtrl+Shift+K`, `Alt+F1`, `Super+N`. The modifiers are `Shift`, `Control`, `Alt`, `Super` and `CmdOrCtrl`, which is Command on macOS and Control elsewhere. An accelerator Nativella cannot read throws a `NativeCallException` that says why.

Another application may hold the shortcut already, or the system may reserve it: then `register()` throws too. Pick combinations with two modifiers, and let the user change them through [settings](settings.md).

> [!NOTE]
> On Linux, global shortcuts are implemented for X11 only. Under a Wayland session, expect them not to fire.
