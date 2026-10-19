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
> Build from the parent directory, as above, so that `dist/` lands outside the project. The executable being written is always left out of the archive, but other files of a `dist/` inside the project, like the builds of other applications, would ship.

## What ships

The whole project directory, `vendor/` included, as a compressed archive, except:

| Left out | |
|---|---|
| `.git`, `.idea`, `node_modules`, `tests` | At the root of the project. |
| `.env.local`, and every path ending with `.local` | Local overrides, which tend to hold secrets. The build lists the ones it leaves out. |

| Shipped empty | |
|---|---|
| `var/cache`, `var/log` | Symfony's cache and logs. |
| `storage/framework/cache`, `storage/framework/sessions`, `storage/framework/views`, `storage/logs` | Laravel's caches, sessions and logs. |

The caches, logs and sessions of your machine stay behind, but the directories ship: the frameworks expect them to exist.

Symbolic links are followed, so the packages Composer links from [path repositories](installation.md#the-php-package) ship as real files.

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

The executable carries the project and PHP: the machine needs neither PHP nor Composer. On Linux, it needs the webview of the system, WebKitGTK, and a glibc as recent as the one of the machine that built the `nativella` command: 2.35 for the releases, built on Ubuntu 22.04, so Ubuntu 22.04 and later, Debian 12 and later.

```bash
sudo apt install libwebkit2gtk-4.1-0
```

An application with a [tray](tray.md) icon needs `libayatana-appindicator3-1` too, which Ubuntu desktops have: without it, the application stops when it shows the icon. On macOS, the executable needs nothing.

## The PHP inside

The executable carries the PHP of the `nativella` command that built it: for the [releases](https://github.com/lyrixx/nativella/releases), PHP 8.5, linked statically, built with [static-php-cli](https://github.com/crazywhalecc/static-php-cli). The Symfony demo makes a 91 MB executable.

It has the extensions a Symfony, Laravel or Doctrine application needs (bcmath, ctype, curl, dom, fileinfo, filter, iconv, intl, mbstring, opcache, openssl, pdo_sqlite, phar, session, simplexml, sodium, sqlite3, tokenizer, xml, xmlreader, xmlwriter, zip, zlib), and no `php.ini`: PHP runs with its defaults, plus the settings Nativella sets (sessions in the cache directory of the application, the time zone of the system). intl carries ICU and its data, for every locale: 36 MB of the executable.

> [!NOTE]
> Installers (`.deb`, `.dmg`, `.msi`), code signing and updates are not there yet. See [Limitations and roadmap](limitations.md#distribution).
