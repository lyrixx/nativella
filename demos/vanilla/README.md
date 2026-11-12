# Snippets: Nativella without a framework

A small desktop app in plain PHP to save text snippets, search them, and copy one to the clipboard
in one click. It has no framework, template engine or build step, only its own router, PHP
templates, one CSS file and one JavaScript file. The database is SQLite through `doctrine/dbal`.

| Feature | Nativella API |
|---|---|
| Copy a snippet | `$native->clipboard->writeText()` |
| Download a snippet as a text file | Nothing: an attachment, which Nativella asks where to save |
| Delete, after a native confirmation | `$native->dialog->confirm()` |
| On phones, delete once the user proved who they are, with a fingerprint, a face or the PIN (`biometrics` permission) | `$native->biometrics->authenticate()`, and the dialog when the phone has no lock |
| Import a text file (button, or File › Import…), on phones too | `$native->dialog->openFile()`, `$native->notification->send()` |
| Import the files dropped on the window (not on phones) | `Event\WindowFilesDropped`, then `$native->page->dispatch()`: the page reloads its list |
| Application menu, global shortcut `CmdOrCtrl+Shift+Space` (not on mobile) | `Event\AppBooted`, `$native->menu->set()`, `$native->globalShortcut->register()` |
| System page | `$native->system->info()`, `$native->screen->all()`, `$native->app->info()`, `Runtime::dataDir()` |

## Running it

```bash
composer install

# From the root of the repository: the desktop app.
cargo run -p nativella -- run demos/vanilla
# Or, with the Castor tasks of the repository:
castor nativella:run vanilla

# In a browser, for development: the native features are hidden.
php -S localhost:8000 -t public

vendor/bin/phpunit
```

The database is `snippets.sqlite` in the data directory Nativella gives the app
(`NATIVELLA_DATA_DIR` moves it), or `var/snippets.sqlite` outside Nativella.

## How it works

- `public/index.php` builds the few services by hand. A native event (an internal
  `POST /_nativella/event`, see `EventFactory::fromGlobals()`) goes to `src/Native/NativeEventHandler.php`;
  anything else goes to `App\Application` (`src/Application.php`), which checks the CSRF token of
  the forms and routes the request.
- On `app.booted`, the schema is created or upgraded by DBAL's comparator (`src/Snippet/SnippetSchema.php`).
  Change the definition, and every user's database follows on the next launch. Under `php -S`, which
  sends no such event, the table is created on the first request.
- Forms post and redirect (303). Validation errors re-render the form with a 422.
- `public/app.js` only enhances: copy without leaving the page, search as you type, refresh the list on
  `nativella:snippets.imported`, focus the search on the global shortcut, show dates in the local time
  zone (PHP stores and renders them in UTC).
- Outside Nativella (`$native->isNative()` is false), the copy and import routes do not exist, their buttons
  are hidden, and deleting asks with the browser's `confirm()`.
- On phones and tablets (`Runtime::isMobile()`), there is no menu, global shortcut or file drop.
  Import works there: the runtime copies the picked file to the application's cache and hands PHP
  its path. Deleting asks who holds the phone (`src/Native/DeleteConfirmation.php`). The page goes under the status bar
  (`viewport-fit=cover`) and keeps clear of it with the `safe-area-inset-*` of `[data-mobile]` in `app.css`.
