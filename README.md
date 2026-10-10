# native-php — spike

A desktop app whose UI is an ordinary PHP application (here Symfony 8), served
**in-process**: a Tauri 2 window hands every request of its `php://` scheme to
PHP through a custom SAPI written in Rust. No HTTP server, no TCP port, no
Node, no Electron.

```
WebKitGTK ──php://localhost/…──▶ Tauri scheme handler (main thread)
                                   │ public file? ─▶ served from disk
                                   ▼
                                 PHP thread ── cookie jar, redirects
                                   │
                                   ▼
                                 custom SAPI (ext-php-rs) ─▶ public/index.php
                                   │                            │
                     native_call() ◀────────────────────────────┘
                                   ▼
                                 Tauri APIs (dialogs, notifications, window…)
```

## Layout

| Path | What |
|---|---|
| `crates/php-runtime` | The runtime, framework-agnostic: the SAPI, the PHP thread, static files, the cookie jar and redirects, and the `native_call()` PHP function. |
| `crates/php-runtime/examples/request.rs` | Serves requests from the command line, without any GUI. |
| `crates/desktop` | The Tauri shell: the `php://` scheme, the native methods (`src/native.rs`), the menu and window events sent to PHP. |
| `crates/desktop/examples/smoke.rs` | Drives the demo page from JavaScript to test WebKit behaviour (POST bodies, cookies, redirects, `fetch()`). |
| `crates/desktop/examples/plain.rs` | The same window without PHP: reproduces the WebKitGTK crash below. |
| `app/` | The Symfony demo. `src/Native/Native.php` wraps `native_call()`; `NativeEventController` receives native events. |

## Running it (Ubuntu 26.04)

```bash
sudo apt install libphp8.5-embed php8.5-dev libclang-dev \
    libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev
(cd app && composer install)

cargo run -p desktop                                          # the app
cargo run -p php-runtime --example request -- app GET / --repeat 5
cargo run -p desktop --example smoke                          # WebKit checks
cargo test -p php-runtime
```

`NATIVE_PHP_APP=/path/to/project` serves another project: anything with a
`public/index.php` works.

Without `libclang-dev`, bindgen (through ext-php-rs) misses clang's builtin
headers (`stddef.h` not found). Pointing it at an extracted
`libclang-common-21-dev` works too:
`BINDGEN_EXTRA_CLANG_ARGS="-isystem …/usr/lib/llvm-21/lib/clang/21/include"`.

## How it works

- **One PHP thread.** Debian's `libphp` is NTS: PHP starts once
  (`sapi_startup` + `php_module_startup`) on a dedicated thread, and requests
  are queued to it. Each one is a full `php_request_startup()` /
  `php_execute_script()` / `php_request_shutdown()` cycle, with OPcache
  shared across requests.
- **A real SAPI**, so PHP behaves as under FPM: `$_SERVER` (process
  environment, then the CGI variables and `HTTP_*` headers), `$_GET`, `$_POST`
  parsed by PHP itself from `read_post`, cookies, `header()`,
  `http_response_code()`. The front controller is untouched. `$_FILES` goes
  through the same path, but uploads were not tested.
- **Static files** of `public/` are served by Rust, path-traversal safe, PHP
  files excepted.
- **PHP → native**: `native_call(string $method, string $jsonPayload): string`,
  a function registered by the runtime (an extension written with
  ext-php-rs). It runs on the PHP thread, so a modal dialog may block it while
  the main thread keeps the UI alive. Errors are thrown as
  `RuntimeException`.
- **Native → PHP**: the host sends `POST /_native/event` as an *internal*
  request, flagged by `$_SERVER['NATIVE_PHP_INTERNAL'] = '1'`. A webview
  request cannot forge it (headers only become `HTTP_*`), so a route
  `condition` is enough to protect the endpoint.

## Measured (debug build of Rust, Symfony `dev`, OPcache on)

| Request | Time |
|---|---|
| First request (container compiled) | 120–260 ms |
| Skeleton 404 page | ~2.5 ms |
| Demo page: Twig, session, event log | ~5 ms |
| Kernel boot to controller, as seen by PHP | ~3 ms |

Every request boots the Symfony kernel: there is no worker mode yet.

## What the spike found

1. **WebKitGTK does no cookie handling for a custom scheme.** It neither
   stores `Set-Cookie` nor sends `Cookie`, so sessions are lost. The runtime
   keeps a cookie jar (RFC 6265 paths, `Max-Age`, `Expires`) on the PHP
   thread, so a response's cookies always reach the next request. NativePHP
   Mobile hits the same wall with `WKWebView` and passes cookies natively too.
2. **WebKitGTK follows no redirect from a custom scheme**, neither for a
   navigation nor for `fetch()`: both get the raw 302. The runtime follows
   redirects itself:
   - navigations get a page that runs `location.replace(target)`, so the
     address bar and history end up as with a real redirect;
   - `fetch()`/XHR get the target's response directly, but then see
     `redirected: false` and the original URL. Turbo, which reads
     `response.redirected`, would need more than this.
3. **No `Sec-Fetch-*` headers** reach a custom scheme. Navigations are told
   apart by `Upgrade-Insecure-Requests`, which `fetch()` never sends.
4. **WebKitGTK 2.52 on Ubuntu 26.04 segfaults in the AT-SPI bridge**
   (`spi_register_object_to_path`) as soon as a page loads, PHP or not
   (`examples/plain.rs`). `NO_AT_BRIDGE=1` avoids it, at the cost of screen
   readers; the app sets it on Linux. Worth reporting upstream.
5. **ext-php-rs** (0.16.1) has an embed SAPI layer good enough for this, with
   rough edges:
   - its `read_cookies` trampoline leaks a C string per request, since PHP
     never frees `cookie_data`; the runtime installs its own callback;
   - nothing applies `ServerContext::init_request_info()`: the SAPI fills
     `SG(request_info)` itself;
   - `RequestInfo::proto_num` documents HTTP/1.1 as 1100; PHP uses 1001.
6. **`exit()` makes `php_execute_script()` report a failure**, and
   `symfony/runtime` always calls it: this is not an error.
7. **Symfony builds absolute URLs as `http://localhost/…`**, as it cannot know
   about `php://`. The runtime maps those back for redirects, but an absolute
   URL in a page leaves the app on Linux and macOS. On Windows, Tauri serves
   custom schemes from `http://php.localhost/`, which should make it
   disappear (not tested).

## Not done

- **Worker mode**: boot the kernel once and loop over requests. ext-php-rs
  ships `worker_request_startup()` / `worker_request_shutdown()`; on the PHP
  side it is a `symfony/runtime` runner, as with FrankenPHP.
- **ZTS and several PHP threads**: today a slow request, or an open native
  dialog, holds back every other request.
- **Distribution**: link a static `libphp` (static-php-cli builds embed
  libraries for desktop targets) with its own `php.ini`, instead of the
  system's `/etc/php/8.5/embed`, then bundle with `tauri build`.
- Cookies are kept in memory only (lost on quit); no CSRF on the demo forms;
  no PHP → page push (Tauri events would do); the PHP glue in `app/src/Native`
  belongs in a bundle.
- Mobile was not tried. The runtime crate has no desktop dependency, but PHP
  still has to be cross-compiled for iOS and Android.
