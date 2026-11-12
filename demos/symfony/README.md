# Notes: Nativella with Symfony

A small note-taking application built with Symfony 8, persisted with Doctrine ORM in SQLite,
running as a desktop application. Notes are written in Markdown and rendered by Twig, with one CSS
file and one JavaScript file, without a build step. The Symfony side of Nativella is the
`nativella/symfony-bundle` package (`packages/symfony`).

| Feature | Nativella API |
|---|---|
| Delete, after a native confirmation | `Nativella\Dialog::confirm()` |
| Copy a note as Markdown, or as rich text | `Nativella\Clipboard::writeText()`, `writeHtml()` |
| Export a note as a Markdown file (not on phones) | `Nativella\Dialog::saveFile()`, `Nativella\Notification::send()` |
| Import Markdown and text files (button, or File › Import…) | `Nativella\Dialog::openFiles()` |
| Import the files dropped on the window | `Event\WindowFilesDropped`, `Nativella\Notification::send()` |
| Context menu on a note: open, edit, pin, duplicate, export, delete | `Nativella\Menu::popup()`, `Event\MenuItemClicked` |
| Application menu: File, Edit, View, Window, Help | `Event\AppBooted`, `Nativella\Menu::set()`, `Event\MenuItemClicked` |
| Tray icon whose tooltip counts the notes | `Nativella\Tray::set()`, `Event\TrayClicked` |
| Global shortcut `CmdOrCtrl+Shift+N` opens a new note | `Nativella\GlobalShortcut::register()`, `Event\GlobalShortcutPressed` |
| Deep link `nativella-notes://notes/<id>` opens a note | `deepLinks` in `composer.json`, `Event\AppUrlsOpened` |
| Always on top, kept between launches, and open at login | `Nativella\Settings`, `Nativella\Window::setAlwaysOnTop()`, `Nativella\App::enableAutostart()` |
| The window is titled after the note | `Nativella\Window::setTitle()` |
| Settings page: the application, the system, the displays, and "Open" on the data directory | `Nativella\App::info()`, `Nativella\System::info()`, `Nativella\Screen::all()`, `Nativella\Shell::openPath()` |
| Every window refreshes when a note changes | `Nativella\Page::dispatch()`, `nativella:notes.changed` in `public/app.js` |
| On phones: add my location to a note (`geolocation` permission) | `Nativella\Geolocation::currentPosition()` |
| On phones: scan a code into a note (`barcodeScanner` permission) | `Nativella\BarcodeScanner::scan()` |
| On phones: share a note | `Nativella\Share::text()` |

## Running it

```bash
composer install

# From the root of the repository: the desktop app.
cargo run -p nativella -- run demos/symfony
# Or, with the Castor tasks of the repository:
castor nativella:run symfony

# In a browser, for development: the native features are hidden.
symfony serve

bin/phpunit
```

In Nativella, the database is `notes.db` in the application's data directory
(`NATIVELLA_DATA_DIR` moves it), migrated when the application starts. Outside Nativella, it is
`var/notes.db`.

## How it works

- `nativella/symfony-bundle` dispatches the native events (`POST /_nativella/event`) through the
  event dispatcher, to the listeners of `src/EventListener`. On `AppBooted`, the bundle migrates the
  database first (`nativella.migrate_on_boot`), then `BootListener` sets the menu, the tray and the
  global shortcut, and opens the main window.
- `App\Native\Desktop` holds the window, the menu and the tray. On macOS, the menu starts with the
  application's submenu, which has Quit; elsewhere, Quit ends the File menu.
- The menu items handled in PHP go through `MenuListener`; links (New note, Settings) and system
  roles (Quit, the Edit and Window menus) are handled by the runtime itself.
- Closing the window leaves the application in the tray. The tray menu, the global shortcut or a
  deep link open it again.
- Markdown is rendered with `html_input: escape` and `allow_unsafe_links: false`
  (`config/packages/twig_extra.yaml`), so a note cannot bring HTML or a `javascript:` link into the
  page.
- Import takes `.md`, `.markdown` and `.txt` files, in UTF-8, up to 1 MB. The others are skipped,
  and the summary of the import names them.
- The sidebar lists the 100 most recently updated notes, pinned first. The search ignores the case
  of ASCII letters only, as SQLite's `LIKE` does.
- On phones and tablets (`Runtime::isMobile()`), there is no menu, tray, global shortcut, context
  menu, export or autostart.
- Inside Nativella, `framework.secret` is the installation's secret, `NATIVELLA_SECRET`, not the
  `APP_SECRET` of the `.env` file, which ships with the application.
- Outside Nativella (`Nativella::isNative()` is false), `nativella_is_native()` hides the native
  buttons, the native routes answer 404, and deleting asks with the browser's `confirm()`.
- Tests use the fake client (`nativella.fake_client: true` in the `test` environment) and assert the
  native calls. `NotesTestCase::sendNativeEvent()` posts an event as the runtime does, and
  `leaveNativella()` runs a test as a browser would.
