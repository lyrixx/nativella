# Installation

The `nativella` command opens a PHP project in a window, and builds it into a standalone executable. It carries its own PHP, so there is nothing else to install, apart from the system webview on Linux. The [releases](https://github.com/lyrixx/nativella/releases) have it for Linux x86_64 and macOS arm64.

## Download

### Linux x86_64

The executable needs glibc 2.35 or later, and the system webview, WebKitGTK 4.1, which it loads when it starts. That library is needed to run it, not to build anything. The standalone executables it builds need the same. Install your distribution's webview:

| Distribution | The webview | For a [tray](tray.md) icon |
|---|---|---|
| Ubuntu 22.04 and later, Debian 12 and later | `sudo apt install libwebkit2gtk-4.1-0` | `libayatana-appindicator3-1`, which Ubuntu desktops have |
| Fedora | `sudo dnf install webkit2gtk4.1` | `libayatana-appindicator-gtk3` |
| RHEL, AlmaLinux, Rocky Linux 10 | `sudo dnf install webkit2gtk4.1`, from [EPEL](https://docs.fedoraproject.org/en-US/epel/) | |
| Arch Linux | `sudo pacman -S webkit2gtk-4.1` | `libayatana-appindicator` |

Elsewhere, install the distribution's WebKitGTK 4.1 package. RHEL, AlmaLinux and Rocky Linux 9 have glibc 2.34, so the releases do not run there. Without the tray library, an application stops when it shows its tray icon.

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

The executable is not signed. Downloaded with a browser, it is quarantined, and `xattr -d com.apple.quarantine nativella` lets it run.

### Checking the download

Each file of a release has a provenance attestation, signed by GitHub, which says that the repository's Release workflow built it from the release tag. The [GitHub CLI](https://cli.github.com) checks it:

```bash
gh attestation verify nativella --repo lyrixx/nativella
```

The same goes for the Android and iOS templates. `SHA256SUMS-<platform>`, next to each platform's files, holds their checksums.

### What is inside

The executable carries PHP 8.5, linked statically, built with [static-php-cli](https://github.com/crazywhalecc/static-php-cli), with the extensions a Symfony, Laravel or Doctrine application needs (see [Building](building.md#the-php-inside) for the list). The machine's PHP, if any, is only for your project: Composer, the console. Check that it runs:

```bash
nativella --version
```

`nativella --help` lists the commands: `run` opens a project in a window, `build` makes a [standalone executable](building.md) of it, `deb` a [Debian package](building.md#debian-packages) on Linux, `android` and `ios` make an [Android or iOS application](mobile.md) of it, `licenses` prints the [licenses](building.md#licenses) of the software the executable contains, PHP and its libraries included.

## The PHP package

The native APIs are a Composer package, `nativella/nativella`. The Symfony bundle and the Laravel package require it, so add the one your application needs.

```bash
composer require nativella/symfony-bundle   # Symfony
composer require nativella/laravel          # Laravel
composer require nativella/nativella        # plain PHP, or another framework
```

[Symfony](symfony.md), [Laravel](laravel.md) and [Plain PHP](plain-php.md) go on from there.

Composer writes a constraint such as `^0.5`, which stays on 0.5.x, because before 1.0, a minor version can break things. The packages and the `nativella` command are released together, with the same version number, so upgrade them together.

To work on Nativella itself, the repository's demos take the packages from `packages/` through a [path repository](https://getcomposer.org/doc/05-repositories.md#path). See [Developing Nativella](development.md).

## Try it without the package

`nativella run` serves any PHP application, package or not. Without the package you get a window, but no native API. It is a good first check:

```bash
nativella run path/to/your/app
```

## Build from source

The releases are built from the repository, with its Docker stack. `castor static-php:build` builds the static PHP, and `castor nativella:build` links `dist/nativella` against it. See [Developing Nativella](development.md#static-php).

## Other platforms

On macOS, the CI builds Nativella and runs its tests and the demos, without a window, and nobody has opened one there yet. Windows is untested. The code has the platform switches Tauri needs (pages are served from `http://php.localhost/` instead of `php://localhost/`), but nobody has built Nativella there yet. It would need PHP for Windows, static or as a library, and WebView2, which ships with Windows 11. Reports and patches are welcome.

On Android, `nativella android` makes a project's APK from its release's template APK; on iOS, `nativella ios` makes an app for the simulator, or, untested, for devices, from a template app of its release. See [Mobile](mobile.md).
