# Your own PHP

The `nativella` command in the [releases](https://github.com/lyrixx/nativella/releases) carries one PHP: PHP 8.5, with the extensions a Symfony, Laravel or Doctrine application needs (see [The PHP inside](building.md#the-php-inside)). An application that needs another extension, like gd or pdo_pgsql, that does without intl for a smaller executable, or that stays on PHP 8.4, builds a `nativella` of its own, from the Nativella repository, with the PHP it chooses. The executables this `nativella` builds carry that PHP.

## Why a whole `nativella`

PHP is not a file next to the executable, which an application could swap. It is linked statically into the `nativella` command, and `nativella build` makes a copy of that command with the project appended. The extensions are compiled into PHP itself, and the runtime's bindings are generated, when it compiles, from the headers of one PHP version. Another PHP means building the command again, with no code to change, because the build links whatever the PHP build contains.

## Choose the PHP

Two constants at the top of `.castor/static-php.php` describe it:

| Constant | In the releases | |
|---|---|---|
| `PHP_RELEASE` | `8.5.12` | The PHP version, to its patch release (see [The version of PHP](development.md#the-version-of-php)). Nativella's PHP packages need 8.4 or later. |
| `EXTENSIONS` | `bcmath,ctype,…,zlib` | The extensions, separated by commas, by their names in [static-php-cli](https://static-php.dev/en/guide/extensions.html), which builds PHP and the libraries they need. |

For PHP 8.4, with gd and without intl:

```php
const EXTENSIONS = 'bcmath,ctype,curl,dom,fileinfo,filter,gd,iconv,mbstring,opcache,openssl,pdo,pdo_sqlite,phar,session,simplexml,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,zip,zlib';

const PHP_RELEASE = '8.4.27';
```

Keep what the application needs. `composer check-platform-reqs --lock --no-dev`, in its directory, lists the extensions its packages require. Then add the ones nobody declares: session, if the application has sessions; pdo_sqlite, for the [database](database.md) in the data directory.

Without intl, the executable is about 36 MB smaller, because intl carries ICU and its data, for every locale. Formatting by locale goes with it: Symfony's polyfills know English only, and Laravel's `Number` helper needs intl.

## Settings

PHP reads Nativella's [`php.ini`](configuration.md#phpini), `crates/runtime/php.ini`, compiled into the command. An application changes most settings with `ini_set()`; the ones PHP reads before the script runs, like the upload limits, or only when it starts, like opcache's memory size, go in that file, before the build.

## Build it

On Linux, the build runs in the repository's Docker stack, which needs Docker and Castor, as in [Developing Nativella](development.md#prerequisites).

```bash
git clone https://github.com/lyrixx/nativella.git
cd nativella
git checkout v0.5.0              # the version of nativella/nativella in your composer.lock
# change the constants of .castor/static-php.php
castor static-php:build
castor nativella:build vanilla
```

`static-php:build` builds PHP as a static library, in the `static` image, Ubuntu 22.04, as for the releases, so the executables run on the same distributions, with glibc 2.35 or later. It then does nothing until the constants change. static-php-cli downloads the sources first, and asks the GitHub API where some are. Without a token, the API may turn it down (`403`). Give it one with `GITHUB_TOKEN=<token> castor static-php:build`: a [fine-grained token](https://github.com/settings/personal-access-tokens/new) with no permission, which reads public repositories only, rather than the one from `gh auth token`, which can do everything you can. Only the download gets it.

`nativella:build` builds `dist/nativella`, gathers the [third-party notices](#licenses) of what it links, then builds the standalone executable of each demo it is given into `dist/`. The vanilla demo, the smallest, checks that the new PHP starts.

On a computer with 24 threads, the first `static-php:build` takes about 7 minutes, downloads included, and the first `nativella:build` 4.

On a Mac, without Docker, the `host` context runs the same tasks with the machine's tools, as the CI's macOS runners do. It needs Rust, the Xcode command line tools, Homebrew, which static-php-cli installs its build tools with, and PHP with Composer, which prepare the demo:

```bash
castor --context=host static-php:build
castor --context=host nativella:build vanilla
```

## Use it

`dist/nativella` is a `nativella` command like the release one, with your PHP. Put it in your `PATH`, or call it by its path.

```bash
path/to/nativella/dist/nativella build notes
```

To check what a `nativella` carries, `nativella licenses` starts with the PHP version, then the libraries it links. In the application, `PHP_VERSION` and `get_loaded_extensions()` say the same, and `phpinfo()` says everything, library versions included.

## Licenses

The third-party notices follow the build. `nativella:build` gathers the licenses static-php-cli writes for PHP and for each library it builds, and `nativella build` writes them next to the executable, as for the releases (see [Licenses](building.md#licenses)). An extension may bring a library under another license, which you then distribute, so check the licenses of what you add, like GMP, under the LGPL 3, for gmp. The paragraph of the notices about LGPL code knows the release libraries only: GNU libiconv, libbcmath and libmbfl.

## On phones

The phones' PHP comes from the same constants. static-php-cli builds for computers only, so Nativella builds it for Android and iOS itself, with options and recipes of its own, in `.castor/static-php-mobile.php` (see [PHP for the phones](development.md#php-for-the-phones)). An extension outside the release list needs its `configure` options there, and each library it brings needs a recipe, or the build stops and says which.

`castor mobile:android-template` then builds a template APK with that PHP, into `target/nativella/`, and `castor mobile:ios-template` builds the iOS template apps, on a Mac. Give them to `nativella android` and `nativella ios` with `--template`. See [Mobile](mobile.md).

## What does not work

- Adding an extension at runtime does not work, neither with `extension=` in a `php.ini` nor with `dl()`. The executable exports no PHP symbol for an extension to link against, because everything is compiled in.
- There is no PHP per application. Each `nativella` command builds with its own PHP only. Letting an application pick its extensions at `nativella build` is on the [roadmap](limitations.md#roadmap).
