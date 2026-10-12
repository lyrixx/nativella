# Installation

Nativella is built from source for now. A Rust toolchain compiles the `nativella` command, which links against the PHP of the system. Ubuntu 26.04 is the platform it is developed and tested on.

## System packages

On Ubuntu 26.04:

```bash
sudo apt install libphp8.5-embed php8.5-dev php8.5-cli php8.5-sqlite3 \
    libclang-dev libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev
```

| Package | What for |
|---|---|
| `libphp8.5-embed` | PHP as a shared library (`libphp.so`): the PHP that Nativella embeds. |
| `php8.5-dev` | The headers of PHP, to compile the runtime against them. |
| `php8.5-cli` | PHP on the command line, for Composer. Keep it on the same version as the embedded PHP. |
| `php8.5-sqlite3` | SQLite and `pdo_sqlite`, for the [database](database.md). |
| `libclang-dev` | The PHP bindings are generated with clang (bindgen, through ext-php-rs). |
| `libwebkit2gtk-4.1-dev` | WebKitGTK, the webview of the windows. |
| `libxdo-dev`, `librsvg2-dev` | Libraries Tauri needs on Linux. |

Add the extensions your application needs: `php8.5-mbstring`, `php8.5-xml`, `php8.5-intl`… Debian enables an extension for every SAPI, including the embedded one: they show up in `/etc/php/8.5/embed/conf.d/`, next to its `php.ini`.

> [!TIP]
> Without `libclang-dev`, bindgen misses the builtin headers of clang (`stddef.h` not found). Pointing it at an extracted `libclang-common-21-dev` works too:
> `BINDGEN_EXTRA_CLANG_ARGS="-isystem …/usr/lib/llvm-21/lib/clang/21/include"`.

## Rust

A recent stable Rust, 1.88 or later: the crates use the 2024 edition. [rustup](https://rustup.rs) installs it.

## The `nativella` command

```bash
git clone https://github.com/lyrixx/nativella.git
cd nativella
cargo install --locked --path crates/nativella
```

`nativella` lands in `~/.cargo/bin`. Check that it runs:

```bash
nativella --version
```

`nativella --help` lists the commands: `run` opens a project in a window, `build` makes a [standalone executable](building.md) of it.

## The PHP package

`nativella/nativella`, and the Symfony and Laravel integrations, are not on Packagist yet. Point Composer at your clone with a path repository, in the `composer.json` of your application:

```json title="composer.json"
{
    "repositories": [
        {
            "type": "path",
            "url": "../nativella/packages/*",
            "options": {
                "versions": {
                    "nativella/laravel": "0.1.x-dev",
                    "nativella/nativella": "0.1.x-dev",
                    "nativella/symfony-bundle": "0.1.x-dev"
                }
            }
        }
    ]
}
```

The packages share one Git repository, so Composer cannot guess their versions: the `versions` option gives them one. Then:

```bash
composer require nativella/nativella:^0.1@dev
```

Composer symlinks path repositories into `vendor/`. That is fine: [`nativella build`](building.md) follows the links and ships what they point to.

## Try it without the package

`nativella run` serves any PHP application, package or not. Without the package you get a window, but no native API. It is a good first check:

```bash
nativella run path/to/your/app
```

## Other platforms

macOS and Windows are untested. The code has the platform switches Tauri needs (on Windows, pages are served from `http://php.localhost/` instead of `php://localhost/`), but nobody has built or run Nativella there yet. You would need the embed SAPI of PHP (`libphp`) for the platform, and its webview: WKWebView ships with macOS, WebView2 with Windows 11. Reports and patches are welcome.
