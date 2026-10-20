# Installation

The `nativella` command opens a PHP project in a window, and builds it into a standalone executable. It carries its own PHP: nothing else to install, apart from the webview of the system on Linux. The [releases](https://github.com/lyrixx/nativella/releases) have it for Linux x86_64 and macOS arm64.

## Download

### Linux x86_64

The executable needs glibc 2.35 or later, and the webview of the system, WebKitGTK 4.1, which it loads when it starts: a library to run it, not to build anything. The standalone executables it builds need the same. Install the webview of your distribution:

| Distribution | The webview | For a [tray](tray.md) icon |
|---|---|---|
| Ubuntu 22.04 and later, Debian 12 and later | `sudo apt install libwebkit2gtk-4.1-0` | `libayatana-appindicator3-1`, which Ubuntu desktops have |
| Fedora | `sudo dnf install webkit2gtk4.1` | `libayatana-appindicator-gtk3` |
| RHEL, AlmaLinux, Rocky Linux 10 | `sudo dnf install webkit2gtk4.1`, from [EPEL](https://docs.fedoraproject.org/en-US/epel/) | |
| Arch Linux | `sudo pacman -S webkit2gtk-4.1` | `libayatana-appindicator` |

Elsewhere, install the WebKitGTK 4.1 package of the distribution. RHEL, AlmaLinux and Rocky Linux 9 have glibc 2.34: the releases do not run there. Without the library of the tray, an application stops when it shows its tray icon.

Then the command:

```bash
curl -fLo nativella https://github.com/lyrixx/nativella/releases/latest/download/nativella-linux-x86_64
chmod +x nativella
sudo mv nativella /usr/local/bin/
```

### macOS arm64

```bash
curl -fLo nativella https://github.com/lyrixx/nativella/releases/latest/download/nativella-macos-aarch64
chmod +x nativella
sudo mv nativella /usr/local/bin/
```

The executable is not signed: downloaded with a browser, it is quarantined, and `xattr -d com.apple.quarantine nativella` lets it run.

### What is inside

The executable carries PHP 8.5, linked statically, built with [static-php-cli](https://github.com/crazywhalecc/static-php-cli), with the extensions a Symfony, Laravel or Doctrine application needs: see [Building](building.md#the-php-inside) for the list. The PHP of the machine, if any, is only for your project: Composer, the console. Check that it runs:

```bash
nativella --version
```

`nativella --help` lists the commands: `run` opens a project in a window, `build` makes a [standalone executable](building.md) of it.

## The PHP package

The native APIs are a Composer package, `nativella/nativella`. The Symfony bundle and the Laravel package require it: add the one your application needs.

```bash
composer require nativella/symfony-bundle   # Symfony
composer require nativella/laravel          # Laravel
composer require nativella/nativella        # plain PHP, or another framework
```

[Symfony](symfony.md), [Laravel](laravel.md) and [Plain PHP](plain-php.md) go on from there.

Composer writes a constraint such as `^0.2`, which stays on 0.2.x: before 1.0, a minor version can break things. The packages and the `nativella` command are released together, with the same version number: upgrade them together.

To work on Nativella itself, the demos of the repository take the packages from `packages/` through a [path repository](https://getcomposer.org/doc/05-repositories.md#path): see [Developing Nativella](development.md).

## Try it without the package

`nativella run` serves any PHP application, package or not. Without the package you get a window, but no native API. It is a good first check:

```bash
nativella run path/to/your/app
```

## Build from source

The releases are built from the repository, with its Docker stack: `castor static-php:build` builds the static PHP, `castor nativella:build` links `dist/nativella` against it. See [Developing Nativella](development.md#static-php).

## Other platforms

On macOS, the CI builds Nativella and runs its tests and the demos, without a window: nobody has opened one there yet. Windows is untested: the code has the platform switches Tauri needs (pages are served from `http://php.localhost/` instead of `php://localhost/`), but nobody has built Nativella there yet. It would need PHP for Windows, static or as a library, and WebView2, which ships with Windows 11. Reports and patches are welcome.

On Android, `nativella android` makes the APK of a project from the template APK of its release. iOS is a build from the repository, for the simulator. See [Mobile](mobile.md).
