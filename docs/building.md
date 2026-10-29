# Building a standalone app

`nativella build` turns a project into a single executable: a copy of the `nativella` command, PHP included, with the project packed inside. Run it, and the application starts: the machine needs no PHP.

```bash
cd path/to/notes
composer install --no-dev --optimize-autoloader
cd ..
nativella build notes
```

```text
Packaging Notes 1.2.0 into dist/notes…
Done: dist/notes
```

```text
nativella build [<project-dir>] [-o <output>]
```

The project directory defaults to the current one. The executable goes to `dist/` in the current directory, named after the [`name`](configuration.md#name) of the application, in lower case with dashes (`.exe` on Windows); `-o` picks another path.

> [!TIP]
> `nativella build` from the project directory works too: the `dist/` it writes to is never packed, so the builds of earlier runs do not ship. With `-o` to another directory of the project, only the executable being written is left out of the archive.

## What ships

The whole project directory, `vendor/` included, as a compressed archive, except:

| Left out | |
|---|---|
| `.git`, `.idea`, `dist`, `node_modules`, `tests` | At the root of the project. `dist` is where `nativella build` writes: a `dist` directory of your own there does not ship either, but `public/dist` does. |
| `.env.local`, and every path ending with `.local` | Local overrides, which tend to hold secrets. The build lists the ones it leaves out. |

| Shipped empty | |
|---|---|
| `var/cache`, `var/log` | Symfony's cache and logs. |
| `storage/framework/cache`, `storage/framework/sessions`, `storage/framework/views`, `storage/logs` | Laravel's caches, sessions and logs. |

The caches, logs and sessions of your machine stay behind, but the directories ship: the frameworks expect them to exist.

Symbolic links are followed, so the packages Composer links from [path repositories](https://getcomposer.org/doc/05-repositories.md#path) ship as real files.

> [!WARNING]
> Everything else ships, and anyone who has the executable can read it: `.env` files, keys, credentials. A desktop application has no secrets from its users. Keep out of the project anything they should not see: [Credentials](#credentials) says where each kind goes.

## Preparing the project

- Install the dependencies without the development ones: `composer install --no-dev --optimize-autoloader`. Whatever is in `vendor/` ships.
- Build the front-end assets, if you have any: `node_modules` stays behind, `public/build` does not.
- Set the production environment. With Symfony, `composer dump-env prod` writes `.env.local.php`, which ships (its name does not end with `.local`) and wins over the `.env` files.
- Avoid caches that hold absolute paths of your machine, like Laravel's `config:cache`: once extracted, the project runs from another directory.

## Credentials

The users of the application can read whatever ships: the project is extracted in clear into their cache directory, and no packaging hides it. Where a credential comes from decides where it goes:

| Credential | Where it goes |
|---|---|
| The secret the framework signs and encrypts with: Symfony's `APP_SECRET`, Laravel's `APP_KEY` | Nowhere: each installation has its own, `NATIVELLA_SECRET`. The [Laravel](laravel.md#the-encryption-key) integration gives it to the framework; with Symfony, a line of `framework.yaml` [does](symfony.md#the-secret). |
| The credentials of the user: a token, the password of an account, a key they bring | Asked for in the application, and kept in the keychain of the system with [`Secrets`](secrets.md). |
| Your own credentials: the key of a paid API, a token with rights on your services | Not in the application: on a server of yours, which keeps them, does the calls, and authenticates the users of the application. |
| Keys made to be public: a Sentry DSN, a publishable Stripe key, a key restricted to one use | In the configuration that ships, like `.env.local.php`. |

A release built by a CI can write the `.env.local.php` of `composer dump-env prod` from the secrets of the CI, just before `nativella build`: what it holds still ships, in clear. Keep it for what may be public.

## At startup

The executable finds the project it carries and extracts it, once per build, into the cache directory of the user:

| Platform | Extracted to |
|---|---|
| Linux | `~/.cache/<identifier>/<build>` (or `$XDG_CACHE_HOME`) |
| macOS | `~/Library/Caches/<identifier>/<build>` |
| Windows | `%LOCALAPPDATA%\<identifier>\<build>` |

The next launches of the same build start right away. A new build extracts again, next to the previous one, and removes it once it runs: when no other instance of the application can be running from there. The data directory stays. Extraction is safe when two instances start at once: each one extracts aside, and the first to finish wins.

Then the application runs exactly as with `nativella run`. Its command line belongs to the application: a second launch forwards it to the running instance as [`AppReopened`](lifecycle.md#a-second-launch).

The [data directory](configuration.md#the-data-directory) is separate from the extracted project: the database and the settings survive updates, as long as the identifier does not change.

## What the machine needs

The executable carries the project and PHP: the machine needs neither PHP nor Composer. On Linux, it needs, to run, what the `nativella` command needs: the webview of the system, WebKitGTK 4.1, the library of the tray if the application shows a [tray](tray.md) icon, and a glibc as recent as the one of the machine that built the `nativella` command, 2.35 for the releases, built on Ubuntu 22.04. The users of your application install them too: [Installation](installation.md#linux-x86_64) lists the packages of each distribution, to repeat in the instructions of your application. On macOS, the executable needs nothing.

## The PHP inside

The executable carries the PHP of the `nativella` command that built it: for the [releases](https://github.com/lyrixx/nativella/releases), PHP 8.5, linked statically, built with [static-php-cli](https://github.com/crazywhalecc/static-php-cli), in the latest patch release of the day of the build: 8.5.11 for 0.4.0. `nativella licenses` starts with it, and `phpinfo()`, in the application, gives the versions of the libraries too. The Symfony demo makes a 91 MB executable: see [Size and memory](#size-and-memory).

It has the extensions a Symfony, Laravel or Doctrine application needs (bcmath, ctype, curl, dom, fileinfo, filter, iconv, intl, mbstring, opcache, openssl, pdo_sqlite, phar, session, simplexml, sodium, sqlite3, tokenizer, xml, xmlreader, xmlwriter, zip, zlib), and the [`php.ini`](configuration.md#phpini) of Nativella only: PHP runs with its defaults, plus the settings Nativella sets (sessions in the cache directory of the application, the time zone of the system). intl carries ICU and its data, for every locale: 36 MB of the executable.

Android and iOS applications carry the same PHP, with the same extensions, built for the phone: see [PHP for the phone](mobile.md#php-for-the-phone).

Another version of PHP, or other extensions, take a `nativella` built with them, from the repository: see [Your own PHP](custom-php.md).

> [!NOTE]
> Linux has [Debian packages](#debian-packages). Other installers (`.dmg`, `.msi`) and code signing are not there yet; standalone executables [update themselves](updates.md). See [Limitations and roadmap](limitations.md#distribution).

## Size and memory

The executables of the [0.4.0 release](https://github.com/lyrixx/nativella/releases/tag/v0.4.0), and the memory the demos take on Linux x86_64, with their first page open (Ubuntu 26.04, GNOME on Wayland, WebKitGTK 2.52):

| | `nativella` | Vanilla demo | Symfony demo | Laravel demo |
|---|---|---|---|---|
| Executable, Linux x86_64 | 86.9 MB | 87.2 MB | 90.8 MB | 91.8 MB |
| The same, compressed (gzip) | 29.5 MB | 29.9 MB | 33.4 MB | 34.4 MB |
| Executable, macOS arm64 | 74.4 MB | 74.7 MB | 78.3 MB | 79.3 MB |
| APK, Android arm64 | 80.2 MB, the template | 80.5 MB | 84.1 MB | 85.0 MB |
| Project, [extracted](#at-startup), on disk | | 4 MB | 42 MB | 49 MB |
| Memory: the application process | | 130 MB | 135 MB | 130 MB |
| Memory: the processes WebKit starts | | 140 MB | 130 MB | 125 MB |
| **Memory: total** | | **270 MB** | **265 MB** | **255 MB** |

The sizes are those GitHub shows, in MB of 1,048,576 bytes.

Nearly all of an executable is the `nativella` command: PHP and its libraries, ICU and its data for intl (36 MB), and the Rust shell around the webview. The project adds its own size, compressed: 0.3 MB for the vanilla demo, 4 to 5 MB for the Symfony and Laravel ones, `vendor/` included. The webview is the one of the system, and is not part of it. A `nativella` with [your own PHP](custom-php.md), without intl, makes smaller executables.

The memory is the PSS of each process (`Pss` in `/proc/<pid>/smaps_rollup`): its own memory, plus its share of what it shares with other processes, like the libraries of the system. The application holds the Rust shell, GTK, the side of WebKit that drives the windows, and PHP. WebKit starts a web process, which renders the pages and runs their JavaScript, a network process, and the sandboxes of both. The pages of the demos are light: heavier ones take more of the memory of WebKit, as in a browser. The RSS of these processes, which counts the shared libraries in full in each one, adds up to about 500 MB. The memory on macOS and on phones is not measured yet.

## Debian packages

On Linux, `nativella deb` makes a Debian package of the project, for Debian, Ubuntu and the distributions made of them: the users install it with their package manager, which brings the webview along, and find the application among the others.

```bash
nativella deb notes
```

```text
Packaging Notes 1.2.0 into dist/notes_1.2.0_amd64.deb…
Done: dist/notes_1.2.0_amd64.deb
```

```text
nativella deb [<project-dir>] [-o <output>]
```

| The package holds | |
|---|---|
| `/usr/bin/<package>` | The executable of `nativella build`. |
| `/usr/share/applications/<package>.desktop` | The entry of the application, with its [deep link schemes](configuration.md#deeplinks). |
| `/usr/share/icons/hicolor/*/apps/<package>.png` | The [icon](configuration.md#icon), from 32 to 512 pixels: give it 512 pixels or more. |
| `/usr/share/doc/<package>/THIRD_PARTY_NOTICES.txt` | The [licenses](#licenses) of the software inside. |

The package is named after the [`name`](configuration.md#name) of the application, in lower case with dashes, and so are its executable and its entry: the desktop matches the windows to the entry, and so to the icon, by that name. Its version is the [`version`](configuration.md#version), which has to start with a digit. composer.json gives the rest: the `description`, the `homepage`, and the first of the `authors` as the maintainer, or the name of the application.

It depends on `libwebkit2gtk-4.1-0` and a glibc 2.35 or later (Ubuntu 22.04, Debian 12), and recommends `libayatana-appindicator3-1`, for a [tray](tray.md) icon. It is made for the processor of the machine that builds it, `amd64` or `arm64`: build it with the `nativella` of the [releases](installation.md), whose PHP is linked statically. A development build of `nativella` links the PHP of its machine, and warns that its packages will not run elsewhere.

## Licenses

The executable contains the software of others, under their licenses: PHP, the libraries it is linked with (OpenSSL, curl, ICU, libxml2, GNU libiconv, libsodium, libzip, SQLite, zlib) and the ones php-src bundles (PCRE2, libmagic, lexbor, uriparser…), and the Rust crates of Nativella (Tauri, wry, tao, ext-php-rs…). Most of these licenses ask that their copyright notices and texts go with every copy, and the PHP License that it says "This product includes PHP software, freely available from <http://www.php.net/software/>".

The `nativella` command of the [releases](https://github.com/lyrixx/nativella/releases) carries these third-party notices, gathered when it is built: `nativella licenses` prints them, and `nativella build` writes them next to the executable it builds.

```text
Packaging Notes 1.2.0 into dist/notes…
Wrote the licenses of the software the application contains to dist/THIRD_PARTY_NOTICES.txt: distribute them with it.
Done: dist/notes
```

To distribute the application:

- ship `THIRD_PARTY_NOTICES.txt` with the executable, in the same archive or installer, or show it in the application, in an "about" page: [`App::thirdPartyNotices()`](app.md#methods) returns it;
- add the licenses of what your project ships: the Composer packages of `vendor/` (`composer licenses` lists them), the front-end libraries.

The libraries of the system the executable loads are not part of it, and need no notice: glibc, libstdc++ and WebKitGTK on Linux, the frameworks of macOS. The releases attach the notices of each platform too, `THIRD_PARTY_NOTICES-<platform>.txt`. A `nativella` built without them, by `cargo build` rather than [`castor nativella:build`](development.md#standalone-executables), says so, and `nativella build` warns.

> [!IMPORTANT]
> Some of the static PHP is under the [GNU Lesser General Public License 2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html), linked statically: GNU libiconv, which the iconv extension and libxml2 use, and, inside php-src, libbcmath (bcmath) and libmbfl (mbstring). The notices give the version of each, and the URL of the source archive it was built from. Among other things, the LGPL asks that whoever gets the application can use it with a modified version of these libraries. What Nativella offers for that: it is open source, and builds from source, PHP included ([`castor static-php:build`, then `castor nativella:build`](development.md#standalone-executables)); the project of an application is an archive appended to that executable, which it [extracts](#at-startup) in clear, and `nativella build` packs again. Nativella cannot tell you what your distribution needs: ask someone who knows these licenses.
