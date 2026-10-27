# How it works

Nativella is three pieces. A desktop shell in Rust, on top of Tauri 2 ([`crates/nativella`](../crates/nativella)). A runtime that runs PHP in-process through a custom SAPI ([`crates/runtime`](../crates/runtime)), which knows nothing of windows. And a PHP package that wraps the native APIs in classes ([`packages/nativella`](../packages/nativella)).

![A request in Nativella: the webview, the Rust scheme handler, the PHP thread](images/architecture.svg)

## A request, step by step

**1. The page asks for a URL.** Windows load `php://localhost/`. `php` is a custom URI scheme the shell registers with Tauri, so the webview hands its requests to Rust instead of the network stack. On Windows, Tauri exposes custom schemes as `http://php.localhost/` instead.

**2. Rust serves files, and queues the rest.** The scheme handler runs on the main thread. If the path names a file of `public/`, other than a PHP file, the runtime reads it from disk and answers right away, with a guessed MIME type. Paths are canonicalized first: anything that resolves outside `public/` is never read from disk. Everything else goes to a queue, with the label of the window that sent it.

**3. The PHP thread runs the front controller.** PHP lives on one dedicated thread. For each request, it does what a web server SAPI does: `php_request_startup()`, `php_execute_script()` on `public/index.php`, `php_request_shutdown()`. The custom SAPI fills `$_SERVER` (the environment of the process, then the CGI variables, the `HTTP_*` headers and the [Nativella variables](configuration.md#what-php-sees)), feeds the body to PHP through `read_post`, so that PHP parses `$_POST` itself, and collects the output, the status code and the headers. The front controller is untouched: it cannot tell it is not behind PHP-FPM.

**4. The response goes back.** Before it does, the runtime stores the cookies the response sets and follows its redirect, if it is one: things a browser does, and WebKit does not do here. The response is then handed back to the webview, from the PHP thread.

The webview never sees a socket. There is no HTTP server in the process and no port to find: nothing else on the machine can reach the application.

## One PHP thread

The PHP of Nativella is built without thread safety (NTS), on the desktop as on the phones. PHP starts once, on its own thread (`sapi_startup()`, then `php_module_startup()`), and stays up for the life of the process. Requests are queued to it and served one at a time, each one a full request cycle, with OPcache shared between them.

It is simple and it is fast enough, with one consequence: a slow request holds back every other one, in every window. So does a [modal dialog](dialogs.md), since PHP waits for its answer. Static files are not affected, as Rust serves them without PHP.

The fix is a thread-safe (ZTS) PHP with several PHP threads, and a worker mode that boots the framework once: see [Limitations and roadmap](limitations.md#roadmap).

## PHP calls native code

The runtime registers one function in PHP, `nativella_call(string $method, string $jsonPayload): string`, as an extension written with [ext-php-rs](https://github.com/davidcole1340/ext-php-rs) and loaded at module startup:

```php
$result = nativella_call('window.setTitle', '{"title": "3 unsaved notes"}'); // '{}'
```

Methods are named `<domain>.<method>`, like `window.setTitle`, and take and return JSON objects with camelCase keys. The PHP package is a thin layer of classes over it: `Nativella\Client\ExtensionClient` encodes, calls and decodes, and `Window`, `Dialog` and friends build the payloads and type the results.

The call runs on the PHP thread, which may block (a dialog waiting for the user) while the main thread keeps the windows alive. The runtime remembers which window made the current request, so that native calls act on it by default. When the native side fails, `nativella_call()` throws a `RuntimeException`, which the package turns into a `Nativella\Exception\NativeCallException`.

## Native code calls PHP

Native events go the other way as internal requests: `POST /_nativella/event`, with a JSON body naming the event, sent by the shell itself. The runtime marks them with `$_SERVER['NATIVELLA_INTERNAL'] = '1'`. A request from a page cannot set a server variable (its headers only ever become `HTTP_*` variables), so checking it is enough to keep the endpoint private. The integrations do the check, then turn the body into an event object: see [Lifecycle and events](lifecycle.md).

The same events reach the pages: the shell evaluates a `window.dispatchEvent(new CustomEvent('nativella:…'))` in each window. `Page::dispatch()` uses the same path for events of your own.

## What WebKit does not do

Serving a web application through a custom scheme, rather than over HTTP, works remarkably well. These are the gaps the spike that started Nativella found in WebKitGTK, and what the runtime does about them.

1. **No cookies.** WebKitGTK neither stores `Set-Cookie` nor sends `Cookie` for a custom scheme, so sessions are lost on every request. The runtime keeps a cookie jar on the PHP thread, honouring paths, `Max-Age` and `Expires` (RFC 6265). Requests are served one at a time, so the cookies of a response always reach the next request. Cookies with an expiry date are saved to the data directory, and outlive the application; session cookies end with it, as in a browser. NativePHP Mobile hits the same wall with WKWebView, and passes cookies natively too.
2. **No redirects.** WebKitGTK follows no redirect from a custom scheme, for a navigation or a `fetch()`: both get the raw `302`. The runtime follows redirects itself, up to 20 of them. A navigation gets a tiny page that runs `location.replace(target)`, so the address and the history end up where a real redirect would put them. A `fetch()` or an XHR gets the response of the target directly, but sees `redirected: false` and the original URL. Turbo, which reads `response.redirected`, needs more than this.
3. **No `Sec-Fetch-*` headers.** Nothing tells a navigation from a `fetch()`, except `Upgrade-Insecure-Requests`, which navigations carry and `fetch()` never sends. That is what the runtime goes by.
4. **No `Host` header.** The runtime sets `HTTP_HOST` to `localhost`, the host of the URL.
5. **A crash in the accessibility bridge.** WebKitGTK 2.52, on Ubuntu 26.04, segfaults in the AT-SPI bridge as soon as a page loads, with or without PHP: [`examples/plain.rs`](../crates/nativella/examples/plain.rs) reproduces it with a plain Tauri window. `NO_AT_BRIDGE=1` avoids it, at the cost of screen readers. Nativella sets it on Linux, unless you set it yourself.
6. **No downloads.** WebKitGTK cannot download from a custom scheme: it drops an attachment, and fails a link with a `download` attribute without a request. Nativella saves them itself, and answers the webview an empty response of a type it cannot show, so that it stays on the page: a `204 No Content`, which keeps the page over HTTP, shows an empty one on a custom scheme. See [Downloads](downloads.md).

## What PHP needed

ext-php-rs (0.16) has an embed SAPI layer good enough for this, with a few rough edges the runtime works around:

- its `read_cookies` callback leaks a C string per request, since PHP never frees `cookie_data`: the runtime installs its own;
- nothing applies `ServerContext::init_request_info()`: the SAPI fills `SG(request_info)` itself;
- `RequestInfo::proto_num` documents HTTP/1.1 as 1100, where PHP uses 1001.

And on the PHP side:

- `exit()` makes `php_execute_script()` report a failure, and `symfony/runtime` always calls it: the runtime does not treat it as an error.
- Frameworks build absolute URLs as `http://localhost/…`, since that is what the SAPI tells them, and nothing listens there. The runtime maps them back when it follows a redirect, and rewrites them to `php://localhost/…` in the URL attributes of HTML and the `url()` of CSS that PHP answers (not in text, scripts or JSON, which are the user's data). A script the shell injects in every page brings back the requests that `fetch()`, XHR and `EventSource` send to `http://localhost/…`. It never follows a redirect to a `javascript:` or `data:` URL, as browsers don't. A navigation to one that slipped through is brought back to the application. On Windows, where pages are served from `http://php.localhost/` over HTTP already, there is nothing to rewrite (untested).
- The other way around, the `Origin` and `Referer` headers of the webview say `php://localhost`: the SAPI hands them to PHP as `http://localhost`, or the same-origin checks of CSRF protections would fail.
- A navigation to another website (`http:`, `https:`, `mailto:`, `tel:`) opens in the browser of the system instead of the window.

## Measured

On the Symfony demo, with a debug build of the Rust code, Symfony in the `dev` environment and OPcache on:

| Request | Time |
|---|---|
| First request, compiling the container | 120–260 ms |
| The 404 page of a Symfony skeleton | ~2.5 ms |
| The demo page: Twig, a session, an event log | ~5 ms |
| Kernel boot to controller, as PHP sees it | ~3 ms |

Every request boots the Symfony kernel: there is no worker mode yet. The runtime prints the time of each request on its standard error output, as `[php] GET php://localhost/ 200 4.8 ms`.

## Standalone executables

`nativella build` appends the project to a copy of the `nativella` executable itself:

```text
[executable][project .tar.gz][manifest JSON][archive length][manifest length][NTVLLA01]
```

At startup, the executable looks for that trailer. When it finds one, it runs the embedded project instead of reading its command line, extracting it once per build into the cache directory of the user. See [Building a standalone app](building.md).
