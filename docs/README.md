# Nativella documentation

Nativella runs a PHP web application as a desktop application: a native window, PHP embedded in the application itself, and no web server in between. These pages go from the first window to the single-file build, and down to how the requests travel.

Nativella is young and experimental. Linux is the platform it is built and tested on; Android and iOS builds are experimental. Read [Limitations and roadmap](limitations.md) before you bet anything on it.

## Start here

- [Introduction](introduction.md): what Nativella is, what it is not, and when it fits.
- [Installation](installation.md): the `nativella` command, with PHP inside, and the PHP package.
- [Getting started](getting-started.md): from an empty directory to a desktop application, in plain PHP.
- [What runs where](platforms.md): every native API, on computers and on phones.
- [Configuration](configuration.md): the `extra.nativella` section of `composer.json`, and the environment.

## Frameworks

- [Symfony](symfony.md): the bundle, the APIs as services, native events as Symfony events.
- [Laravel](laravel.md): the package, the facades, native events as Laravel events.
- [Plain PHP](plain-php.md): no framework, `Nativella::create()` and the event factory.

## Guides

- [Lifecycle and events](lifecycle.md): booting, native events, closing windows, quitting.
- [Database](database.md): SQLite in the data directory, migrated when the application starts.
- [Downloads](downloads.md): attachments and `download` links, saved where the user chooses.
- [Testing](testing.md): the fake client, and running the application outside Nativella.
- [Building a standalone app](building.md): `nativella build`, and what ships in the executable.
- [Updates](updates.md): standalone executables that update themselves from your releases.
- [Mobile](mobile.md): `nativella android` and `nativella ios`, what changes on a phone, what is tested.

## Native APIs

- [App](app.md): name and version, directories, quit and restart, autostart, badge.
- [Windows](windows.md): open, move, resize, hide and close windows.
- [Menus](menus.md): the application menu and context menus.
- [Tray](tray.md): an icon in the system tray, with its menu.
- [Dialogs](dialogs.md): message boxes, confirmations and file pickers.
- [Notifications](notifications.md): desktop notifications.
- [Clipboard](clipboard.md): read and write text and HTML.
- [Global shortcuts](global-shortcuts.md): keyboard shortcuts that work in the background.
- [Settings](settings.md): a key-value store for preferences.
- [Secrets](secrets.md): tokens and passwords in the keychain of the system.
- [Timers](timers.md): wake PHP up for periodic work.
- [Child processes](child-processes.md): start processes, with their output as events.
- [Phone APIs](phone-apis.md): haptics, geolocation, biometrics, a barcode scanner and the share sheet, on phones.
- [Shell](shell.md): open URLs and files with the applications of the system.
- [Screens](screens.md): displays, work areas and the cursor.
- [System](system.md): the operating system, the locale and the theme.
- [Power](power.md): the battery, the power source, waking from sleep.
- [Pages and JavaScript](javascript.md): `nativella:*` DOM events, `fetch()`, context menus.

## Under the hood

- [How it works](how-it-works.md): the scheme handler, the SAPI, the PHP thread, and what WebKit does not do.
- [Limitations and roadmap](limitations.md): what is missing, what is untested, what comes next.

## Contributing

- [Developing Nativella](development.md): the Docker stack, the Castor tasks, the quality checks and the CI.
