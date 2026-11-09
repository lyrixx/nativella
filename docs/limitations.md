# Limitations and roadmap

Nativella is experimental. This page lists what does not work, what is untested and what is planned.

## Platforms

| Platform | Status |
|---|---|
| Linux | Developed and tested on Ubuntu 26.04, with WebKitGTK 2.52. The release executables run on Ubuntu 22.04 and 24.04, Debian 12 and 13 too. |
| macOS | Built and tested by the CI (arm64), without a window: nobody has opened one there yet. |
| Windows | Untested. Pages would be served from `http://php.localhost/`, as Tauri does for custom schemes there. |
| Android | Experimental: the three demos run on an Android 16 emulator (arm64 builds, and the Symfony demo built for x86_64). The CI opens the Symfony demo in an emulator. See [Mobile](mobile.md#what-is-tested). |
| iOS | Experimental: the CI makes the Symfony demo app for the simulator with `nativella ios`, and opens it there. No device has run an app for devices yet. See [Mobile](mobile.md#what-is-tested). |

### Linux

- Screen readers do not work. WebKitGTK 2.52 crashes in its accessibility bridge as soon as a page loads, so Nativella disables the bridge (`NO_AT_BRIDGE=1`). Only the application is affected: the [child processes](child-processes.md) it starts and the browser it opens keep theirs. This should be reported upstream.
- Tray icons report no click. Only their menu works.
- Downloads made by pages need the first window. A file a page makes, from a `blob:` or `data:` URL (an export built in JavaScript), is silently cancelled once the first window the application opened is closed, until a window with the same label opens again. WebKitGTK asks the windows in the order they opened, and Tauri answers no for a closed one. [Downloads](downloads.md) that PHP answers are not affected.
- Menu roles are Nativella's own. GTK implements few of them, so Nativella does, on the focused page and window. See [Menus](menus.md#roles).
- Global shortcuts need X11. Under Wayland, expect them not to fire.
- There is no primary display under Wayland, so `Screen::primary()` returns null.

## One request at a time

- Every request of every window goes through the same PHP thread, one after the other. A slow request holds back the others, and so does an open [dialog](dialogs.md). Files in `public/` are served without PHP and keep loading.
- There is no worker mode. The framework boots on every request, as under PHP-FPM, which takes about 3 ms for Symfony on top of your code.
- PHP does no background work. It only runs to answer a request, a native event or a [timer](timers.md), and a timer stands in for cron. A queue worker can run as a [child process](child-processes.md) if the machine has PHP's command line.
- `putenv()` is not thread-safe. PHP runs in the window's process, so `putenv()` changes the environment that the window's threads can read at any time, and the C library does not guard against both at once. The application may crash, though rarely. Laravel calls it for every `.env` variable on every request until its configuration is cached. `Illuminate\Support\Env::disablePutenv()`, at the top of `bootstrap/app.php`, leaves them in `$_ENV` and `$_SERVER` only.

## The web layer

- Sessions end with the application, as in a browser, unless their cookie has an expiry date. A cookie with one is kept in the [data directory](configuration.md#the-data-directory). Keep what must last in the [database](database.md) or the [settings](settings.md).
- A `fetch()` that is redirected gets the right response, but `response.redirected` is `false` and `response.url` is the original URL. A form sent to a `307` or a `308` shows the target page at the form's address.
- A `fetch()` gives up after 20 redirects, but a page that redirects to itself keeps reloading, where a browser would stop with an error.
- Absolute URLs are only partly handled. PHP believes it is served from `http://localhost/`. The runtime rewrites such URLs in the HTML URL attributes and the CSS `url()` that PHP answers, and brings back the requests `fetch()`, `XMLHttpRequest` and `EventSource` send there. An absolute URL in a static file of `public/`, or one a script uses otherwise (the `src` of an image), still points to where nothing listens. Prefer relative URLs there.
- Frames of other websites do not work. Nativella cannot tell a frame's navigation from its window's, because wry, Tauri's webview library, does not say which frame navigates. An `<iframe>` of another website (a video, a map, a payment form) opens that website in the browser and stays empty. An `<iframe>` of `http://localhost/…` takes its window to that application page. Frames of the application's own pages work. Windows tells its frames apart, but this is untested.
- Static files are read whole by the thread that receives the request, and `Range` requests are not supported. That is fine for styles, scripts and images, but not for a large video.
- Responses are not streamed. A response reaches the page once PHP is done with it, so `flush()` sends nothing, a streamed response arrives whole, and server-sent events (`text/event-stream`) arrive all at once, at the end. A stream that never ends holds the PHP thread, and every window, forever, because `connection_aborted()` is always false and `sleep()` does not count against `max_execution_time` on Linux. The runtime logs a warning for a `text/event-stream` response. Push to the page with [events](javascript.md#from-php-to-the-page-events) instead. Request and response bodies are held whole in memory too, which is fine for a form or a photo but not for a large video.
- Uploads break on Linux. wry, Tauri's webview library, stops reading a request body at the first short read, so a form with a file reaches PHP cut after its first part and `$_FILES` holds `UPLOAD_ERR_PARTIAL`. The fix is proposed upstream in [tauri-apps/wry#1911](https://github.com/tauri-apps/wry/pull/1911). Until it ships, [file dialogs](dialogs.md) give PHP a file's path instead. Other platforms are untested.

## The native side

- Clicking a [notification](notifications.md#clicks) reaches PHP on Linux only.
- PHP gets no event before a window closes. Only [its page](javascript.md#unsaved-changes) can keep it open.
- [Power](power.md) knows the battery and the power source, but not the screen lock or the thermal state, which NativePHP's `PowerMonitor` reports.

## Distribution

- The PHP is fixed. The executable carries [the PHP of the `nativella` command](building.md#the-php-inside), and an Android application carries its template's PHP, with the same extensions, those of the build. It only reads Nativella's [`php.ini`](configuration.md#phpini). Using another PHP means [building `nativella`](custom-php.md) again.
- Installers are few. Linux has [Debian packages](building.md#debian-packages), but there is no `.dmg` or `.msi` yet and no code signing. Standalone executables [update themselves](updates.md).
- Do not put secrets in the project. Everything in it ships, readable by anyone who has the executable.

## Roadmap

Next:

- Worker mode, to boot the framework once and loop over requests. ext-php-rs ships `worker_request_startup()` and `worker_request_shutdown()`. On the PHP side, it is a `symfony/runtime` runner, as with FrankenPHP.
- ZTS and several PHP threads, so that a slow request or a dialog does not hold back the others.
- macOS and Windows need building, testing and fixing.
- Let an application pick its PHP extensions and `php.ini` settings at `nativella build`, without [building `nativella`](custom-php.md), to add an extension or to leave intl and the ICU data out of a smaller executable.
- `.dmg` and `.msi` installers.
- On mobile, more [phone APIs](phone-apis.md) (photos and videos, push notifications), an iOS app tried on a device, and PCRE's JIT on Android. See [Mobile](mobile.md).

If you find something that is not on this page, [open an issue](https://github.com/lyrixx/nativella/issues).
