# Building a standalone app

`nativella build` turns a project into a single executable: a copy of the `nativella` command, PHP included, with the project packed inside. Run it and the application starts, with no PHP needed on the machine. On a Mac, it makes an app instead, `dist/Notes.app` (see [macOS apps](#macos-apps)).

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

The project directory defaults to the current one. The executable goes to `dist/` in the current directory, named after the application's [`name`](configuration.md#name), in lower case, with dashes in place of everything but ASCII letters, digits, `+` and `.`. `Notes/Pro` makes `dist/notes-pro` (`.exe` on Windows, and `dist/Notes-Pro.app` on macOS, whose apps keep their name). A name that starts with none of these letters and digits, or of one character, gets an `app-` prefix, and a name without any, such as "笔记", leaves the last segment of the [`identifier`](configuration.md#identifier) to name the file. `-o`, or `--output`, picks another path.

> [!TIP]
> `nativella build` from the project directory works too. The `dist/` it writes to is never packed, so earlier builds do not ship. With `-o` to another directory of the project, only the executable being written is left out of the archive.

## What ships

The whole project directory, `vendor/` included, as a compressed archive, except:

| Left out | |
|---|---|
| `.git`, `.idea`, `dist`, `node_modules`, `tests` | At the project root. `dist` is where `nativella build` writes, so a `dist` directory of your own there does not ship either, but `public/dist` does. |
| `.env.local`, and every path ending with `.local` | Local overrides, which tend to hold secrets. The build lists the ones it leaves out. |
| `.pem`, `.key` and `.p8` files that hold a private key; `.json` files and files without an extension, of 64 KiB or less, that hold one; `.p12`, `.pfx`, `.ppk`, `.jks`, `.keystore` and `.mobileprovision` files; `auth.json`, at the project root | Private keys, such as an `id_ed25519` or the key of a Google service account, keystores, provisioning profiles, and Composer's tokens. These file types are left out anywhere in the project, but the certificates of a `.pem` file without a private key ship, like those of `composer/ca-bundle`. The build lists the ones it leaves out. |
| `.db`, `.sqlite` and `.sqlite3` files, and their `-journal`, `-wal` and `-shm` files | Your machine's SQLite databases. The application makes its own, in its [data directory](configuration.md#the-data-directory). The build lists the ones it leaves out. |

| Shipped empty | |
|---|---|
| `var` | What Symfony writes, its cache and its logs, and what the application writes there, like a SQLite database. Symfony makes the directories it needs. |
| `storage/app`, `storage/framework/cache`, `storage/framework/sessions`, `storage/framework/views`, `storage/logs` | What Laravel writes: its files, caches, sessions and logs. |

What your application wrote on your machine stays behind, but the directories ship, because the frameworks expect them to exist.

```text
Packaging Notes 1.2.0 into dist/notes…
Left out .env.local: local overrides never ship. Set the production environment in a file that does, like the .env.local.php of `composer dump-env prod` with Symfony.
Left out auth.json, config/jwt/private.pem: private keys and credentials never ship, as anyone who has the application could read them.
Done: dist/notes
```

Symbolic links are followed, and what they lead to ships as real files:

- a package Composer links from a [path repository](https://getcomposer.org/doc/05-repositories.md#path), a directory with a `composer.json`, ships without its own `.git`, `node_modules`, `tests` and `vendor`;
- a link to a directory of the project ships as that directory does: not at all if it is left out, empty if it ships empty. Laravel's `public/storage`, which leads to `storage/app/public`, ships as an empty directory;
- a link to a directory that holds it stops the build, with an error that names it;
- a link to a file or a directory that does not exist is left out, and listed: `Left out <paths>: symbolic links to a file or a directory that does not exist.`

> [!WARNING]
> Everything else ships, and anyone who has the executable can read it: `.env` files, and the keys and credentials in any other file. A desktop application has no secrets from its users. Keep anything they should not see out of the project. [Credentials](#credentials) says where each kind goes.

## Preparing the project

- Install the dependencies without the development ones: `composer install --no-dev --optimize-autoloader`. Whatever is in `vendor/` ships.
- Build the front-end assets, if you have any. `node_modules` stays behind, `public/build` does not.
- Set the production environment. With Symfony, `composer dump-env prod` writes `.env.local.php`, which ships (its name does not end with `.local`) and wins over the `.env` files.
- Avoid caches that hold absolute paths from your machine, like Laravel's `config:cache`, because once extracted, the project runs from another directory.

## Credentials

The application's users can read whatever ships, because the project is extracted in clear into their cache directory and no packaging hides it. Where a credential comes from decides where it goes:

| Credential | Where it goes |
|---|---|
| The secret the framework signs and encrypts with: Symfony's `APP_SECRET`, Laravel's `APP_KEY` | Nowhere. Each installation has its own, `NATIVELLA_SECRET`. The [Laravel](laravel.md#the-encryption-key) integration gives it to the framework; with Symfony, a line in `framework.yaml` [does](symfony.md#the-secret). |
| The user's credentials: a token, an account password, a key they bring | Asked for in the application, and kept in the system keychain with [`Secrets`](secrets.md). |
| Your own credentials: a paid API's key, a token with rights on your services | On a server of yours, not in the application. The server keeps them, makes the calls, and authenticates the application's users. |
| Keys made to be public: a Sentry DSN, a publishable Stripe key, a key restricted to one use | In the configuration that ships, like `.env.local.php`. |

A release built by a CI can write the `.env.local.php` of `composer dump-env prod` from the CI's secrets, just before `nativella build`. What it holds still ships, in clear, so keep it for what may be public.

## At startup

The executable finds the project it carries and extracts it, once per build, into the user's cache directory:

| Platform | Extracted to |
|---|---|
| Linux | `~/.cache/<identifier>/<build>` (or `$XDG_CACHE_HOME`) |
| macOS | `~/Library/Caches/<identifier>/<build>` |
| Windows | `%LOCALAPPDATA%\<identifier>\<build>` |

A macOS app keeps its project apart from its executable, in its resources, and extracts it the same way.

The next launches of the same build start right away. A build of other files, or by another version of Nativella, extracts again, next to the previous one, and removes it once it runs, when no other instance of the application can be running from there. The data directory stays. Extraction is safe when two instances start at once, because each one extracts aside and the first to finish wins.

At each start, the application checks that every file of the build is still there, and extracts the build again when some are missing, as after a cleaner went through the cache. So an application must not delete the files it ships, and what it writes among them does not last: write to the [data directory](configuration.md#the-data-directory).

Then the application runs exactly as with `nativella run`. Its command line belongs to the application, and a second launch forwards it to the running instance as [`AppReopened`](lifecycle.md#a-second-launch).

The [data directory](configuration.md#the-data-directory) is separate from the extracted project, so the database and the settings survive updates, as long as the identifier does not change.

## Reproducible builds

The same project, built by the same `nativella`, makes the same executable, byte for byte. The project archive keeps neither the dates nor the owners of its files, and `<build>`, at startup, is a digest of it. APKs are reproducible too, because all their entries are dated 1981-01-01, as Android's build tools do. A Debian package dates its files, and an iOS app for devices its signature, with `SOURCE_DATE_EPOCH`, in seconds since 1970, when it is set, as [reproducible builds](https://reproducible-builds.org/docs/source-date-epoch/) ask, and with the build time otherwise.

```bash
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct) nativella deb notes
```

## What the machine needs

The executable carries the project and PHP, so the machine needs neither PHP nor Composer. On Linux, it needs what the `nativella` command needs to run: the system webview, WebKitGTK 4.1, the tray library if the application shows a [tray](tray.md) icon, and a glibc as recent as that of the machine that built the `nativella` command, 2.35 for the releases, built on Ubuntu 22.04. The desktops of Ubuntu 24.04 and later, of Debian 12 and later with GNOME, and of Fedora Workstation 43 already have WebKitGTK 4.1, so their users install nothing. The others install what is missing. [Installation](installation.md#linux-x86_64) lists the packages for each distribution, and which desktops have them, to repeat in your application's instructions. On macOS, the executable needs nothing.

## The PHP inside

The executable carries the PHP of the `nativella` command that built it. For the [releases](https://github.com/lyrixx/nativella/releases), that is PHP 8.5, linked statically, built with [static-php-cli](https://github.com/crazywhalecc/static-php-cli), in the patch release the repository pins ([The version of PHP](development.md#the-version-of-php)), 8.5.12 for 0.6.0. `nativella licenses` starts with it, and `phpinfo()`, in the application, also gives the library versions. The Symfony demo makes a 91 MB executable (see [Size and memory](#size-and-memory)).

It has the extensions a Symfony, Laravel or Doctrine application needs (bcmath, ctype, curl, dom, fileinfo, filter, iconv, intl, mbstring, opcache, openssl, pdo, pdo_sqlite, phar, session, simplexml, sodium, sqlite3, tokenizer, xml, xmlreader, xmlwriter, zip, zlib), and reads only Nativella's [`php.ini`](configuration.md#phpini). PHP runs with its defaults, plus the settings Nativella sets (errors in the logs rather than the page, uploads up to 512 MB, sessions in the application's cache directory, the system time zone). intl carries ICU and its data, for every locale, which makes up 36 MB of the executable.

Android and iOS applications carry the same PHP, with the same extensions, built for the phone. See [PHP for the phone](mobile.md#php-for-the-phone).

Another PHP version, or other extensions, take a `nativella` built with them, from the repository. See [Your own PHP](custom-php.md).

> [!NOTE]
> Linux has [Debian packages](#debian-packages), macOS [apps](#macos-apps), signed ad hoc. Other installers (`.dmg`, `.msi`), signing by a developer and Apple's notarization are not there yet; standalone executables and apps [update themselves](updates.md). See [Limitations and roadmap](limitations.md#distribution).

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

Nearly all of an executable is the `nativella` command: PHP and its libraries, ICU and its data for intl (36 MB), and the Rust shell around the webview. The project adds its own size, compressed: 0.3 MB for the vanilla demo, 4 to 5 MB for the Symfony and Laravel ones, `vendor/` included. The webview is the system's, and is not part of it. A `nativella` with [your own PHP](custom-php.md), without intl, makes smaller executables.

The memory is the PSS of each process (`Pss` in `/proc/<pid>/smaps_rollup`): its own memory, plus its share of what it shares with other processes, like the system libraries. The application holds the Rust shell, GTK, the side of WebKit that drives the windows, and PHP. WebKit starts a web process, which renders the pages and runs their JavaScript, a network process, and the sandboxes of both. The demo pages are light; heavier ones take more WebKit memory, as in a browser. The RSS of these processes, which counts the shared libraries in full in each one, adds up to about 500 MB. The memory on macOS and on phones is not measured yet.

## macOS apps

On a Mac, `nativella build` makes an app: a `.app` directory that Finder shows as one file, with its name and its icon, and that opens as any other application.

```text
Packaging Notes 1.2.0 into dist/Notes.app…
Done: dist/Notes.app
```

| The app holds | |
|---|---|
| `Contents/MacOS/<name>` | The `nativella` command, as it is, but its signature. |
| `Contents/Resources/nativella.bundle` | The project, packed as in an executable. |
| `Contents/Resources/AppIcon.icns` | The [icon](configuration.md#icon), from 16 to 1,024 pixels. |
| `Contents/Info.plist` | The application's `name`, `identifier` and `version`, as three integers (`1.2.3-beta.1` makes `1.2.3`), the reasons macOS gives the user when the pages ask for the camera, the microphone or the location, for the [`permissions`](configuration.md#permissions) it asks for, and the schemes of its [deep links](configuration.md#deeplinks), which macOS registers once the app has been opened or moved by Finder. |
| `Contents/_CodeSignature/CodeResources` | The seal of the resources, which the signature binds. |

The app is signed ad hoc, as `codesign -s -` signs: macOS checks that nothing changed in it, but no developer signs it, and Apple did not notarize it. It opens on the Mac that built it. Downloaded from the Internet, it opens once the user allows it, the first time: macOS says that it cannot check the app, then opens it from the context menu's Open on macOS 14, and from "Open Anyway", in the Privacy & Security settings, on macOS 15 and later. Tell your users, until Nativella signs with a developer certificate.

To publish it, archive it, because a directory does not download: `ditto -c -k --keepParent dist/Notes.app notes.zip` makes a ZIP file, which Finder opens with a double click, and the `.tar.gz` that [updates](updates.md#apps-of-macos) take works too. Ask your users to move the app to their Applications folder before they open it. macOS runs a downloaded app that has not moved from a read-only copy elsewhere, where it cannot update itself.

## Debian packages

On Linux, `nativella deb` makes a Debian package of the project, for Debian, Ubuntu and the distributions based on them. Users install it with their package manager, which brings the webview along, and find the application among the others.

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
| `/usr/share/applications/<package>.desktop` | The application's entry, with its [deep link schemes](configuration.md#deeplinks). |
| `/usr/share/icons/hicolor/*/apps/<package>.png` | The [icon](configuration.md#icon), from 32 to 512 pixels. Give it 512 pixels or more, and 4096 at most: `nativella deb` warns about a smaller one, and refuses a larger one. |
| `/usr/share/doc/<package>/THIRD_PARTY_NOTICES.txt` | The [licenses](#licenses) of the software inside. |

The package is named after the application's [`name`](configuration.md#name), in lower case with dashes, and so are its executable and its entry, because the desktop matches the windows to the entry, and so to the icon, by that name. Its version is the [`version`](configuration.md#version), with a pre-release after a `~` rather than a `-`, and its other dashes as dots. `1.2.0-beta.1` becomes `1.2.0~beta.1`, which Debian sorts before `1.2.0`, as a pre-release, and `1.2.0-beta.1+build-7` becomes `1.2.0~beta.1+build.7`. composer.json gives the rest: the `description`, the `homepage`, and the first of the `authors` as the maintainer. Without them, or without a composer.json, the description and the maintainer are the application's name.

It depends on `libwebkit2gtk-4.1-0` and a glibc 2.35 or later (Ubuntu 22.04, Debian 12), and recommends `libayatana-appindicator3-1`, for a [tray](tray.md) icon. `apt install ./<package>.deb` installs them along with it where they are missing. It is made for the processor of the machine that builds it, `amd64` or `arm64`. Build it with the `nativella` of the [releases](installation.md), whose PHP is linked statically. A development build of `nativella` links its machine's PHP, and warns that its packages will not run elsewhere.

## Licenses

The executable contains other people's software, under their licenses: PHP, the libraries it is linked with (OpenSSL, curl, ICU, libxml2, GNU libiconv, libsodium, libzip, SQLite, zlib) and the ones php-src bundles (PCRE2, libmagic, lexbor, uriparser…), and Nativella's Rust crates (Tauri, wry, tao, ext-php-rs…). Most of these licenses ask that their copyright notices and texts go with every copy, and the PHP License that it says "This product includes PHP software, freely available from <http://www.php.net/software/>".

The `nativella` command of the [releases](https://github.com/lyrixx/nativella/releases) carries these third-party notices, gathered when it is built. `nativella licenses` prints them, and `nativella build` writes them next to the executable it builds. It replaces the notices of a previous build, but refuses to write over another file of that name: move it, or choose another output directory.

```text
Packaging Notes 1.2.0 into dist/notes…
Wrote the licenses of the software the application contains to dist/THIRD_PARTY_NOTICES.txt: distribute them with it.
Done: dist/notes
```

To distribute the application:

- ship `THIRD_PARTY_NOTICES.txt` with the executable, in the same archive or installer, or show it in the application, in an "about" page: [`App::thirdPartyNotices()`](app.md#methods) returns it;
- add the licenses of what your project ships: the Composer packages of `vendor/` (`composer licenses` lists them), the front-end libraries.

The system libraries the executable loads are not part of it, and need no notice: glibc, libstdc++ and WebKitGTK on Linux, the macOS frameworks. The releases also attach each platform's notices, `THIRD_PARTY_NOTICES-<platform>.txt`. A `nativella` built without them, by `cargo build` rather than [`castor nativella:build`](development.md#standalone-executables), says so, and `nativella build` warns.

> [!IMPORTANT]
> Some of the static PHP is under the [GNU Lesser General Public License 2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html), linked statically: GNU libiconv, which the iconv extension and libxml2 use, and, inside php-src, libbcmath (bcmath) and libmbfl (mbstring). The notices give the version of each, and the URL of the source archive it was built from. They name the archive of every other library PHP is linked with too. Among other things, the LGPL asks that whoever gets the application can use it with a modified version of these libraries. For that, Nativella is open source and builds from source, PHP included ([`castor static-php:build`, then `castor nativella:build`](development.md#standalone-executables)). An application's project is an archive appended to that executable, which it [extracts](#at-startup) in clear and `nativella build` packs again. Nativella cannot tell you what your distribution needs, so ask someone who knows these licenses.
