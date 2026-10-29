# Introduction

Nativella turns a PHP web application into a desktop application. Your framework, routes, templates and forms stay as they are. Nativella gives them a native window, menus, dialogs, notifications and a tray icon, and runs PHP inside the application process.

## What happens when you run it

`nativella run` opens a window on `php://localhost/`. When the page asks for a URL, nothing touches the network: the request is handed over in memory to an embedded PHP interpreter, which runs your `public/index.php` through a custom SAPI, much as PHP-FPM would. The response goes straight back to the window.

There is no web server, no TCP port, nothing listening on `127.0.0.1`. Other programs on the machine cannot reach your application, because there is nothing to reach.

The window is the system's own webview: WebKitGTK on Linux, WKWebView on macOS, WebView2 on Windows. The shell around it is written in Rust, on top of [Tauri 2](https://tauri.app). There is no bundled Chromium, no Node.js, no Electron.

```text
window ── php://localhost/notes ──▶ Rust (Tauri) ──▶ PHP, same process ──▶ public/index.php
```

[How it works](how-it-works.md) follows a request all the way.

## What you write

PHP. Native features are plain PHP objects:

```php
use Nativella\Nativella;

$native = Nativella::create();

$native->window->setTitle('3 unsaved notes');
$native->notification->send('Export finished', 'notes.zip is in your Downloads.');
```

What happens on the native side (a menu click, a global shortcut, files dropped on a window) comes back to PHP as typed event objects, and to the page's JavaScript as DOM events. See [Lifecycle and events](lifecycle.md).

Not every API works on every platform: some are for computers only, a few for phones only. [What runs where](platforms.md) lists each one.

## Any PHP application

Nativella needs a `public/index.php` front controller, and nothing else. Symfony, Laravel, Slim, a hand-written router, an old-school `index.php` with a `switch`: if it runs behind nginx and PHP-FPM, it should run in Nativella. The front controller is not modified, `$_SERVER`, `$_GET`, `$_POST`, cookies, sessions and `header()` work as usual.

An application that knows nothing about Nativella still opens in a window. The PHP package, `nativella/nativella`, is what gives it the native APIs, and the integrations wire them into your framework:

- [Symfony](symfony.md): a bundle registers the APIs as services, dispatches native events through the event dispatcher and migrates the database when the application starts.
- [Laravel](laravel.md): a package does the same with the service container, events and Eloquent migrations.
- [Plain PHP](plain-php.md): `Nativella::create()`, and a few lines at the top of the front controller.

## State lives on the user's machine

A desktop application has no server to deploy to and no database server to connect to. Each Nativella application gets its own data directory, and the framework integrations keep an SQLite database there, migrated when the application starts. See [Database](database.md).

## One file to hand over

`nativella build` writes a single executable that carries your project and PHP itself, linked statically: your users install neither PHP nor Composer. On Linux, the webview of the system is all it needs. See [Building a standalone app](building.md): the executable of the Symfony demo weighs 91 MB, and takes 265 MB of memory with its window open ([Size and memory](building.md#size-and-memory)).

## On a phone, too

The same project builds into an Android application, and into an iOS one, experimentally, without the tools of either platform: PHP is linked into the application, the pages are served the same way, and the APIs that make sense on a phone work there, with [phone APIs](phone-apis.md) of their own (haptics, geolocation, biometrics, barcode scanner, share sheet, NFC); the pages use the camera and the microphone themselves, with `getUserMedia()`. The three demos run on an Android emulator; the CI builds the Symfony one for Android and iOS on every change, and opens it in an Android emulator and in the iOS simulator. See [Mobile](mobile.md).

## When it fits

Nativella is a good fit for:

- internal tools and back offices that deserve a window rather than a browser tab;
- local-first applications whose data belongs on the user's machine: notes, inventories, catalogues, small CRMs;
- a PHP team that wants a desktop application without learning a second stack.

It is not a good fit, yet, for:

- an application that must ship on macOS or Windows today: both are untested;
- an application for the stores of Android or iOS: phone builds are experimental, and no iOS device has run one yet;
- long requests or many busy windows: one PHP thread serves every request, one at a time.

## Status

Nativella is experimental. It started as a spike to answer one question (can a Tauri window talk to PHP without an HTTP server?) and the answer turned into this project. The APIs may change, and some corners are rough. [Limitations and roadmap](limitations.md) lists them honestly.

Nativella is open source, under the MIT license.
