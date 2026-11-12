# How it works

Nativella has three parts: a desktop shell in Rust, on top of Tauri 2 ([`crates/nativella`](../crates/nativella)), a runtime that runs PHP in-process through a custom SAPI and knows nothing of windows ([`crates/runtime`](../crates/runtime)), and a PHP package that wraps the native APIs in classes ([`packages/nativella`](../packages/nativella)).

![A request in Nativella: the webview, the Rust scheme handler, the PHP thread](images/architecture.svg)

## A request, step by step

1. The page asks for a URL. Windows load `php://localhost/`. `php` is a custom URI scheme the shell registers with Tauri, so the webview hands its requests to Rust instead of the network stack. On Windows and Android, Tauri exposes custom schemes as `http://php.localhost/` instead.

2. Rust serves files and queues the rest. The scheme handler runs on the main thread. If the path names a file of `public/`, other than a PHP file, the runtime reads it from disk and answers right away, with a guessed MIME type. A path can only go down from `public/`, so `..` is refused, encoded or not. Symlinks are followed wherever they point, as web servers do, since the developer made them (Laravel's `storage:link`), but never to a PHP file. Everything else goes to a queue, with the label of the window that sent it.

3. The PHP thread runs the front controller. PHP lives on one dedicated thread. If that thread is gone, the request gets a `503`. For each request, it does what a web server SAPI does: `php_request_startup()`, `php_execute_script()` on `public/index.php`, `php_request_shutdown()`. The custom SAPI fills `$_SERVER` (the environment of the process, then the CGI variables, the `HTTP_*` headers and the [Nativella variables](configuration.md#what-php-sees)), feeds the body to PHP through `read_post`, so that PHP parses `$_POST` itself, and collects the output, the status code and the headers. The front controller is untouched and cannot tell it is not behind PHP-FPM.

4. The response goes back. Before it does, the runtime stores the cookies the response sets and follows its redirect, if it is one, because WebKit does neither here. The response is then handed back to the webview from the PHP thread.

The process has no HTTP server and opens no port, so nothing else on the machine can reach the application.

## One PHP thread

Nativella's PHP is built without thread safety (NTS), on desktop and on phones. PHP starts once, on its own thread (`sapi_startup()`, then `php_module_startup()`), and stays up for the life of the process. Requests are queued to it and served one at a time, each one a full request cycle, with OPcache shared between them.

This is simple and fast enough, but a slow request holds back every other one, in every window. So does a [modal dialog](dialogs.md), since PHP waits for its answer. Static files are not affected, as Rust serves them without PHP.

The fix is a thread-safe (ZTS) PHP with several PHP threads, and a worker mode that boots the framework once. See [Limitations and roadmap](limitations.md#roadmap).

## PHP calls native code

The runtime registers one function in PHP, `nativella_call(string $method, string $jsonPayload): string`, as an extension written with [ext-php-rs](https://github.com/davidcole1340/ext-php-rs) and loaded at module startup:

```php
$answer = nativella_call('window.setTitle', '{"title": "3 unsaved notes"}'); // '{"result":{}}'
```

Methods are named `<domain>.<method>`, like `window.setTitle`, and take and return JSON objects with camelCase keys. The answer is `{"result": {…}}`, or `{"error": {"kind": "…", "message": "…"}}` when the native side fails: the function itself never throws. The PHP package is a thin layer of classes over it: `Nativella\Client\ExtensionClient` encodes, calls and decodes, and `Window`, `Dialog` and friends build the payloads and type the results.

The call runs on the PHP thread, which may block (a dialog waiting for the user) while the main thread keeps the windows alive. The runtime remembers which window made the current request, so that native calls act on it by default. The package turns an error into a `Nativella\Exception\NativeCallException`, with its `kind` (a `Nativella\Exception\ErrorKind`: `Failed`, `InvalidPayload`, `NotFound`, `Timeout` or `Unavailable`) and its `reason`.

## Native code calls PHP

Native events go the other way as internal requests: `POST /_nativella/event`, with a JSON body naming the event, sent by the shell itself. The runtime marks them with `$_SERVER['NATIVELLA_INTERNAL'] = '1'`. A request from a page cannot set a server variable (its headers only ever become `HTTP_*` variables), so checking it is enough to keep the endpoint private. The integrations do the check, then turn the body into an event object. See [Lifecycle and events](lifecycle.md).

The same events reach the pages: the shell evaluates a `window.dispatchEvent(new CustomEvent('nativella:…'))` in each window. `Page::dispatch()` uses the same path for events of your own.

## What WebKit does not do

Serving a web application through a custom scheme rather than over HTTP works well. These are the gaps found in WebKitGTK, most of them by the proof of concept that started Nativella, and what the runtime does about them.

1. WebKitGTK neither stores `Set-Cookie` nor sends `Cookie` for a custom scheme, so sessions are lost on every request. The runtime keeps a cookie jar on the PHP thread, honouring paths, `Max-Age` and `Expires` (RFC 6265). Requests are served one at a time, so the cookies of a response always reach the next request. Cookies with an expiry date are saved to the data directory, and outlive the application; session cookies end with it, as in a browser. NativePHP Mobile hits the same wall with WKWebView, and passes cookies natively too.
2. WebKitGTK follows no redirect from a custom scheme, for a navigation or a `fetch()`. Both get the raw `302`. The runtime follows redirects itself, up to 20 of them. A navigation gets a tiny page that runs `location.replace()` with the whole URL of the target, so the address and the history end up where a real redirect would put them. A window sent from page to page more than 20 times in a row gets a `508`, as a `fetch()` does. A `fetch()` or an XHR gets the response of the target directly, but sees `redirected: false` and the original URL. Turbo, which reads `response.redirected`, needs more than this. A form sent to a `307` or a `308`, which must reach the target with its method and body, gets its response directly too, at the address of the form.
3. There are no `Sec-Fetch-*` headers, so only `Upgrade-Insecure-Requests`, which navigations carry and `fetch()` never sends, tells a navigation from a `fetch()`. The runtime relies on it.
4. There is no `Host` header. The runtime sets `HTTP_HOST` to the host of the URL, `localhost`.
5. WebKitGTK 2.52, on Ubuntu 26.04, segfaults in the AT-SPI bridge as soon as a page loads, with or without PHP. [`examples/plain.rs`](../crates/nativella/examples/plain.rs) reproduces it with a plain Tauri window. `NO_AT_BRIDGE=1` avoids it, at the cost of screen readers. Nativella sets it on Linux, unless you set it yourself.
6. WebKitGTK cannot download from a custom scheme. It drops an attachment, and fails a link with a `download` attribute without a request. Nativella saves them itself, and answers the webview with an empty response of a type it cannot show, so that it stays on the page. A `204 No Content`, which keeps the page over HTTP, shows an empty one on a custom scheme. See [Downloads](downloads.md).

## What PHP needed

ext-php-rs (0.16) has an embed SAPI layer good enough for this, with a few rough edges the runtime works around:

- its `read_cookies` callback leaks a C string per request, since PHP never frees `cookie_data`, so the runtime installs its own;
- nothing applies `ServerContext::init_request_info()`, so the SAPI fills `SG(request_info)` itself.

And on the PHP side:

- `sapi_activate()` sets the protocol of every request to HTTP/1.0, whatever the SAPI said, so the runtime sets HTTP/1.1 back in its SAPI's `activate` hook, which PHP calls next. With HTTP/1.0, `header('Location: …')` answers a `DELETE` with a `302`, which the runtime follows with a `DELETE` again. With HTTP/1.1, it answers a `303`, followed with a `GET`, as under PHP-FPM.
- `exit()` makes `php_execute_script()` report a failure, and `symfony/runtime` always calls it, so the runtime does not treat it as an error.
- Frameworks build absolute URLs as `http://localhost/…`, since that is what the SAPI tells them, and nothing listens there. The runtime maps them back when it follows a redirect, and rewrites them to `php://localhost/…` in what PHP answers: the URL attributes of HTML tags, the candidates of `srcset`, and the `url()` of CSS, in a style sheet, a `style` attribute or a `<style>` element. It leaves the rest alone, as the user's data: text, comments, form values, JSON, and what `script`, `textarea` and `title` elements hold. A script the shell injects in every page brings back the requests that `fetch()`, XHR and `EventSource` send to `http://localhost/…`. It never follows a redirect to a `javascript:` or `data:` URL, as browsers don't. A navigation to one that slipped through is brought back to the application. On Windows and Android, where pages are served from `http://php.localhost/` over HTTP already, there is nothing to rewrite (untested on Windows).
- The webview's `Origin` and `Referer` headers say `php://localhost`. The SAPI hands them to PHP as `http://localhost`, otherwise the same-origin checks of CSRF protections would fail.
- A navigation to another website (`http:`, `https:`, `mailto:`, `tel:`) opens in the system browser instead of the window. So does a redirect to one, an OAuth login for instance, and the window then stays on a page that says so, with a link back.

## Measured

These figures are those of the [proof of concept](https://github.com/lyrixx/nativella/tree/850aa2df4b9f15c680b81ca07696914744565496) that started Nativella, as its README gives them. They were not measured again since. Its demo was a small Symfony application, run with a debug build of the Rust code, Symfony in the `dev` environment and OPcache on:

| Request | Time |
|---|---|
| First request, compiling the container | 120–260 ms |
| The 404 page of a Symfony skeleton | ~2.5 ms |
| Its demo page: Twig, a session, an event log | ~5 ms |
| Kernel boot to controller, as PHP sees it | ~3 ms |

Every request boots the Symfony kernel, as there is no worker mode yet. To measure your own application, read the standard error output, where the runtime prints the time of each request, as `[php] GET php://localhost/ 200 4.8 ms`.

## Standalone executables

`nativella build` appends the project to a copy of the `nativella` executable itself:

```text
[executable][project .tar.gz][manifest JSON][archive length][manifest length][NTVLLA01]
```

At startup, the executable looks for that trailer. When it finds one, it runs the embedded project instead of reading its command line, extracting it once per build into the user's cache directory, and again when files of it went missing there. See [Building a standalone app](building.md).
