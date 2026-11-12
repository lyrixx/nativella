# Introduction

Nativella turns a PHP web application into a desktop application. Your framework, routes, templates and forms stay as they are. Nativella gives them a native window, menus, dialogs, notifications and a tray icon, and runs PHP inside the application process.

## What happens when you run it

`nativella run` opens a window on `php://localhost/`. When the page asks for a URL, the request does not go through the network. It is handed over in memory to an embedded PHP interpreter, which runs your `public/index.php` through a custom SAPI, much as PHP-FPM would, and the response goes straight back to the window.

Nothing listens on `127.0.0.1`, so other programs on the machine cannot reach your application.

The window is the system webview: WebKitGTK on Linux, WKWebView on macOS, WebView2 on Windows. The shell around it is written in Rust, on top of [Tauri 2](https://tauri.app), and the application does not ship Chromium, Node.js or Electron.

```text
window ── php://localhost/notes ──▶ Rust (Tauri) ──▶ PHP, same process ──▶ public/index.php
```

[How it works](how-it-works.md) follows a request all the way.

## What you write

You write PHP. Native features are plain PHP objects:

```php
use Nativella\Nativella;

$native = Nativella::create();

$native->window->setTitle('3 unsaved notes');
$native->notification->send('Export finished', 'notes.zip is in your Downloads.');
```

What happens on the native side (a menu click, a global shortcut, files dropped on a window) comes back to PHP as typed event objects, and to the page's JavaScript as DOM events. See [Lifecycle and events](lifecycle.md).

Not every API works on every platform. Some are for computers only and a few for phones only. [What runs where](platforms.md) lists each one.

## Any PHP application

Nativella needs a `public/index.php` front controller and nothing else. An application that runs behind nginx and PHP-FPM should run in Nativella, whether it uses Symfony, Laravel, Slim, a hand-written router or an old-school `index.php` with a `switch`. The front controller is not modified, and `$_SERVER`, `$_GET`, `$_POST`, cookies, sessions and `header()` work as usual.

An application that knows nothing about Nativella still opens in a window. The PHP package, `nativella/nativella`, is what gives it the native APIs, and the integrations wire them into your framework:

- [Symfony](symfony.md): a bundle registers the APIs as services, dispatches native events through the event dispatcher and migrates the database when the application starts.
- [Laravel](laravel.md): a package does the same with the service container, events and Eloquent migrations.
- [Plain PHP](plain-php.md): `Nativella::create()`, and a few lines at the top of the front controller.

## State lives on the user's machine

A desktop application has no server to deploy to and no database server to connect to. Each Nativella application gets its own data directory, and the framework integrations keep an SQLite database there, migrated when the application starts. See [Database](database.md).

## One file to hand over

`nativella build` writes a single executable that carries your project and a statically linked PHP, so your users install neither PHP nor Composer. On Linux, it only needs the system webview. The Symfony demo executable weighs 91 MB and takes 265 MB of memory with its window open. See [Building a standalone app](building.md) and [Size and memory](building.md#size-and-memory).

## On a phone, too

The same project builds, experimentally, into an Android application and an iOS one, without either platform's tools. PHP is linked into the application and the pages are served the same way. The APIs that make sense on a phone work there, and [phone APIs](phone-apis.md) add haptics, geolocation, biometrics, a barcode scanner, the share sheet and NFC. Pages use the camera and the microphone themselves, with `getUserMedia()`. The three demos run on an Android emulator. On every change that can break them, the CI builds the Symfony demo for Android and iOS, and opens it in an Android emulator and in the iOS simulator. See [Mobile](mobile.md).

## When it fits

Nativella is a good fit for:

- internal tools and back offices that deserve a window rather than a browser tab;
- local-first applications whose data belongs on the user's machine: notes, inventories, catalogues, small CRMs;
- a PHP team that wants a desktop application without learning a second stack.

It is not a good fit, yet, for:

- an application that must ship on macOS or Windows today: both are untested;
- an application for the Android or iOS stores: phone builds are experimental, and no iOS device has run one yet;
- long requests or many busy windows: one PHP thread serves every request, one at a time.

[Compared to NativePHP and Symfony UX](comparison.md) says when another tool fits better.

## Where it comes from

Nativella started as a proof of concept of PHP, Rust and Tauri in one process, to find out if a Tauri window could talk to PHP without an HTTP server. It could, and a Symfony page with Twig and a session took about 5 ms. The [proof of concept](https://github.com/lyrixx/nativella/tree/850aa2df4b9f15c680b81ca07696914744565496) is the first commit of the repository. Its README lists what WebKitGTK does not do on a custom scheme, the rough edges of ext-php-rs and what was left to do. Worker mode and several PHP threads, which it left undone, are still on the [roadmap](limitations.md#roadmap).

## Status

Nativella is experimental. The APIs may change, and some corners are rough. [Limitations and roadmap](limitations.md) lists them.

Nativella is open source, under the MIT license.
