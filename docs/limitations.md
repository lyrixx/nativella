# Limitations and roadmap

Nativella is experimental. This page lists what does not work, what is untested and what is planned, so that you find out here rather than in production.

## Platforms

| Platform | Status |
|---|---|
| Linux | Developed and tested on Ubuntu 26.04, with WebKitGTK 2.52. The executables of the releases run on Ubuntu 22.04 and 24.04, Debian 12 and 13 too. |
| macOS | Built and tested by the CI (arm64), without a window: nobody has opened one there yet. |
| Windows | Untested. Pages would be served from `http://php.localhost/`, as Tauri does for custom schemes there. |
| Android | Experimental: the three demos run on an Android 16 emulator (arm64 builds). See [Mobile](mobile.md). |
| iOS | Experimental: the CI builds the Symfony demo for the simulator and opens it there. See [Mobile](mobile.md#limitations). |

### Linux

- **No screen readers.** WebKitGTK 2.52 crashes in its accessibility bridge as soon as a page loads, so Nativella disables the bridge (`NO_AT_BRIDGE=1`). Worth reporting upstream.
- **Tray icons report no click**: only their menu works.
- **Menu roles are Nativella's own.** GTK implements few of them, so Nativella does, on the focused page and window. See [Menus](menus.md#roles).
- **Global shortcuts need X11**: under Wayland, expect them not to fire.
- **No primary display under Wayland**: `Screen::primary()` returns null.

## One request at a time

- **One PHP thread.** Every request of every window goes through the same PHP thread, one after the other. A slow request holds back the others, and so does an open [dialog](dialogs.md). Files of `public/` are served without PHP, and keep loading.
- **No worker mode.** The framework boots on every request, as under PHP-FPM: about 3 ms for Symfony, on top of your code.
- **No background work in PHP.** PHP only runs to answer a request, a native event or a [timer](timers.md): a timer stands in for cron. A queue worker can run as a [child process](child-processes.md), with PHP's command line on the machine.

## The web layer

- **Sessions end with the application**, as in a browser, unless their cookie has an expiry date: a cookie with one is kept in the [data directory](configuration.md#the-data-directory). Keep what must last in the [database](database.md) or the [settings](settings.md).
- **Redirects and `fetch()`.** A `fetch()` that is redirected gets the right response, but `response.redirected` is `false` and `response.url` is the original URL.
- **Redirect loops.** A `fetch()` gives up after 20 redirects, but a page that redirects to itself keeps reloading, where a browser would stop with an error.
- **Absolute URLs.** PHP believes it is served from `http://localhost/`. The runtime rewrites such URLs in the URL attributes of HTML and the `url()` of CSS that PHP answers, not elsewhere: an absolute URL in JSON, in a script, or in a static file of `public/` still points to where nothing listens. Prefer relative URLs there.
- **Static files are read whole**, by the thread that receives the request, and `Range` requests are not supported: fine for styles, scripts and images, not for a large video.
- **Uploads are untested.** `$_FILES` goes through the same path as any POST body, but nobody has tried it yet. [File dialogs](dialogs.md) give PHP the path of a file instead.

## The native side

- Clicking a [notification](notifications.md#clicks) reaches PHP on Linux only.
- There is no event before a window closes: closing cannot be prevented, or confirmed.
- A menu item cannot be updated on its own: set the menu again.
- [Power](power.md) knows the battery and the power source, not the screen lock or the thermal state, as NativePHP's `PowerMonitor` does.

## Distribution

- **PHP on the machine, by default.** [`nativella build`](building.md) packs the project, not PHP: the machine needs `libphp`, the extensions and the `php.ini` of the system, and WebKitGTK. Unless the `nativella` command itself was [built against a static PHP](building.md#without-php-on-the-machine).
- **No installers.** No `.deb`, `.dmg` or `.msi`, no code signing, no updates.
- **No secrets.** Everything in the project ships, readable by anyone who has the executable.

## Roadmap

In progress:

- **Static PHP by default.** Static linking works on Linux ([Building](building.md#without-php-on-the-machine)), but it is a separate build: release executables of Nativella should come with it, and with a `php.ini` of their own. Then bundle with `tauri build`.

Next:

- **Worker mode**: boot the framework once, and loop over requests. ext-php-rs ships `worker_request_startup()` and `worker_request_shutdown()`; on the PHP side, it is a `symfony/runtime` runner, as with FrankenPHP.
- **ZTS and several PHP threads**, so that a slow request or a dialog does not hold back the others.
- **macOS and Windows**: build, test, fix.

- **Mobile**: more [phone APIs](phone-apis.md) (camera, sharing, push notifications), PHP builds of Nativella's own, and iOS. See [Mobile](mobile.md).

Found something that is not on this page? [Open an issue](https://github.com/lyrixx/nativella/issues).
