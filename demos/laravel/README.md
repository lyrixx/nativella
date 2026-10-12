# Tasks: Nativella with Laravel

A small to-do list built with Laravel 13, persisted with Eloquent in SQLite, running as a desktop
application. Blade views and one hand-written CSS file (`public/app.css`): no Vite, no npm, no build
step. The Laravel side of Nativella is the `nativella/laravel` package (`packages/laravel`).

| Feature | Nativella API |
|---|---|
| Delete, after a native confirmation | `Nativella\Dialog::confirm()` |
| A notification when the last open task is done | `Nativella\Notification::send()`, from `TaskObserver` |
| Copy the list as Markdown | `Nativella\Clipboard::writeText()` |
| Export the list as Markdown (button, or File › Export…) | `Nativella\Dialog::saveFile()` |
| Open at login, in Settings | `Nativella\App::enableAutostart()`, `disableAutostart()`, stored with `Nativella\Settings` |
| Application menu: File › New task (`CmdOrCtrl+N`), Export…, Settings…, Quit; Edit; Window | `Event\AppBooted`, `Nativella\Menu::set()`, `Event\MenuItemClicked` |
| Tray icon whose tooltip counts the open tasks | `Nativella\Tray::set()`, refreshed by `TaskObserver` |
| Global shortcut `CmdOrCtrl+Alt+Shift+T` brings the window back | `Nativella\GlobalShortcut::register()`, `Event\GlobalShortcutPressed` |

## Running it

```bash
composer setup    # composer install, .env, APP_KEY, database/database.sqlite

# From the root of the repository: the desktop app.
cargo run -p nativella -- run demos/laravel

# In a browser, for development: the native features are hidden.
php artisan serve

php artisan test
```

In Nativella, the database is `database.sqlite` in the data directory of the application
(`NATIVELLA_DATA_DIR` moves it), migrated when the application starts. `NATIVELLA_DATA_DIR=… php
artisan tinker` works on that database too. Outside Nativella, it is `database/database.sqlite`.

Without a window, through the runtime:

```bash
cargo run -p nativella-runtime --example request -- demos/laravel \
    POST /_nativella/event --internal --data '{"name":"app.booted","payload":{}}' \
    --server NATIVELLA_DATA_DIR=$HOME/.local/share/nativella-tasks
cargo run -p nativella-runtime --example request -- demos/laravel \
    GET / --server NATIVELLA_DATA_DIR=$HOME/.local/share/nativella-tasks
```

## How it works

- `nativella/laravel` routes the native events (`POST /_nativella/event`) to Laravel's event
  dispatcher: the listeners of `app/Listeners` are discovered as usual. On `AppBooted`, the package
  migrates the database first, then the application sets its menu (`App\Desktop\ApplicationMenu`), its
  tray (`App\Desktop\TaskTray`) and its global shortcut.
- `App\Observers\TaskObserver` keeps the tray in step with the tasks, and celebrates the last one done.
- The menu items handled in PHP are the cases of `App\Desktop\MenuAction`; links (New task, Settings…)
  and system roles (Quit, the Edit and Window menus) are handled by the runtime itself.
- Closing the window leaves the application in the tray. The tray menu or the global shortcut open it
  again (`App\Desktop\MainWindow`): `Window::open('main')` without options, the runtime starts from
  `extra.nativella.window` in `composer.json`.
- On phones and tablets (`Runtime::isMobile()`), there is no menu, tray, global shortcut or autostart.
- Inside Nativella, `nativella/laravel` encrypts with the secret of the installation, not the `APP_KEY`
  of the `.env` file, which ships with the application.
- Outside Nativella (`Nativella::isNative()` is false), `@native` hides the native buttons, the routes
  behind `EnsureNative` do not exist, and deleting asks with the browser's `confirm()`.
- Tests fake the runtime with `Nativella::fake()` and assert the native calls.
