# Building a standalone app

`nativella build` turns a project into a single executable: a copy of the `nativella` command with the project packed inside. Run it, and the application starts.

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
| `.env.local`, and every path ending with `.local` | Local overrides, which tend to hold secrets. |

| Shipped empty | |
|---|---|
| `var/cache`, `var/log` | Symfony's cache and logs. |
| `storage/framework/cache`, `storage/framework/sessions`, `storage/framework/views`, `storage/logs` | Laravel's caches, sessions and logs. |

The caches, logs and sessions of your machine stay behind, but the directories ship: the frameworks expect them to exist.

Symbolic links are followed, so the packages Composer links from [path repositories](installation.md#the-php-package) ship as real files.

> [!WARNING]
> Everything else ships, and anyone who has the executable can read it: `.env` files, keys, credentials. A desktop application has no secrets from its users. Keep out of the project anything they should not see.

## Preparing the project

- Install the dependencies without the development ones: `composer install --no-dev --optimize-autoloader`. Whatever is in `vendor/` ships.
- Build the front-end assets, if you have any: `node_modules` stays behind, `public/build` does not.
- Set the production environment. With Symfony, `composer dump-env prod` writes `.env.local.php`, which ships (its name does not end with `.local`) and wins over the `.env` files.
- Avoid caches that hold absolute paths of your machine, like Laravel's `config:cache`: once extracted, the project runs from another directory.

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

## What the machine still needs

By default, the executable carries your project, not PHP: it uses the PHP and the webview of the system, so the target machine needs their runtime packages, without the `-dev` ones:

```bash
sudo apt install libphp8.5-embed php8.5-sqlite3 libwebkit2gtk-4.1-0
```

plus the PHP extensions your application uses, and `libayatana-appindicator3-1` for a [tray](tray.md) icon, which Ubuntu desktops have: without it, the application stops when it shows the icon. The embedded PHP reads the `php.ini` of the system, `/etc/php/8.5/embed/php.ini`.

## Without PHP on the machine

Built with a static PHP, the `nativella` command makes executables that carry their own PHP: the machine only needs the webview (`libwebkit2gtk-4.1-0` on Linux) and a glibc as recent as the one of the build machine.

The `nativella` of the [releases](https://github.com/lyrixx/nativella/releases) is built this way, so the executables it makes are too. On Linux, it is built on Ubuntu 22.04, for its glibc, 2.35: they run on Ubuntu 22.04 and later, Debian 12 and later. From the repository, with [its Docker stack](development.md):

```bash
castor static-php:build              # PHP and its libraries, with static-php-cli, on Ubuntu 22.04
castor nativella:build --static      # dist/nativella, and the demos, linked against it
dist/nativella build path/to/project -o dist/my-app
```

The Symfony demo makes a 66 MB executable this way. That PHP has the extensions a Symfony, Laravel or Doctrine application needs (bcmath, ctype, curl, dom, fileinfo, filter, iconv, mbstring, opcache, openssl, pdo_sqlite, phar, session, simplexml, sodium, sqlite3, tokenizer, xml, xmlreader, xmlwriter, zip, zlib), but not intl, and no `php.ini`: PHP runs with its defaults, plus the settings Nativella sets (sessions in the cache directory of the application, the time zone of the system). See [Developing Nativella](development.md#static-php).

> [!NOTE]
> Installers (`.deb`, `.dmg`, `.msi`), code signing and updates are not there yet. See [Limitations and roadmap](limitations.md#distribution).
