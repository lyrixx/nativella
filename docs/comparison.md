# Compared to NativePHP and Symfony UX

Two projects stand next to Nativella. [NativePHP](https://nativephp.com) also runs PHP inside a desktop or phone application. [Symfony UX Native](https://symfony.com/bundles/ux-native/current/index.html), the part of Symfony UX that reaches phones, leaves PHP on your server and puts a native shell around your website.

They make different bets:

- NativePHP turns a Laravel application into a native one, with Electron on computers and native views on phones. It is the mature one, and the one to pick today for Windows, macOS or the stores.
- Symfony UX Native keeps your website where it is, and gives it an app on iOS and Android.
- Nativella takes a PHP application as it is, whatever its framework, and runs it on the user's machine, in the system webview, without a web server. It is the youngest of the three.

All are open source, under the MIT license. NativePHP adds paid extras: Bifrost, its build and deployment service, and premium plugins.

This page compares Nativella 0.5, NativePHP Desktop 2.4, NativePHP Mobile 4.7 and Symfony UX Native 3.7, as of November 2026. What it says about the others comes from their documentation and sources, and from the [measures](#size-memory-and-cpu) below. Both move fast, so check their documentation before you decide.

## At a glance

| | Nativella | NativePHP Desktop | NativePHP Mobile | Symfony UX Native |
|---|---|---|---|---|
| Status | Experimental | Production-ready, says [its roadmap](https://nativephp.com/docs/desktop/2/getting-started/status) | Made for the stores | Experimental |
| Platforms | Linux and macOS, not every API of macOS tried. Windows untested, Android and iOS experimental (see [Platforms](limitations.md#platforms)) | Windows, macOS, Linux | iOS, Android | iOS, Android |
| Applications | Any PHP application with a `public/index.php` | Laravel | Laravel | Symfony |
| Where PHP runs | On the device, in the application's process | On the device, in a process of its own | On the device, in the application | On your server |
| How the interface reaches PHP | In memory, no port | HTTP, between two local web servers | Shared memory, no network | HTTP, over the network |
| Interface | Your pages, in the system webview | Your pages, in Chromium | Blade rendered as SwiftUI and Jetpack Compose views; a web view on request | Your pages, in a web view, with native navigation |
| Without a network | Works | Works | Works | The pages come from your server |
| Tools to build | The `nativella` command, without Xcode, the Android SDK or a Mac | PHP and Node.js 22 or later | Xcode on a Mac with Apple silicon and Android Studio, or Bifrost | Xcode and Android Studio, as the shells are your own projects |
| Signing and stores | Android and iOS apps are signed, but none is ready for the stores (see [Mobile](mobile.md#what-is-tested)). macOS apps are signed ad hoc, not notarized. Nothing on Linux and Windows | Signing and notarization, for macOS and Windows | `native:package`, for the App Store and Google Play | Those of your native projects |
| Updates | [Standalone executables](updates.md) update themselves | Automatic, through its `publish` command | Through the stores, or over the air with Bifrost | Pages at each deployment, without the stores. The shell through the stores |

## NativePHP

It is the closest project: PHP on the user's device, a static build of it from [static-php-cli](https://github.com/crazywhalecc/static-php-cli) on computers, and a local SQLite database, migrated when the application starts.

### On a computer

- NativePHP is built on Laravel, and needs it. Nativella takes any PHP application with a front controller, and has integrations for [Symfony](symfony.md) and [Laravel](laravel.md).
- NativePHP ships Electron, so Chromium. Pages render the same everywhere, and every application carries a browser. Nativella uses the system webview, so it has nothing to carry, but one engine per platform, each with its gaps. See [What WebKit does not do](how-it-works.md#what-webkit-does-not-do).
- NativePHP starts a web server on each side of the application, and they talk over HTTP with a key made at each start, which [its security guide](https://nativephp.com/docs/desktop/2/digging-deeper/security) tells you never to bypass. Nativella has no port to protect, because a request stays in the memory of the process.
- For background work, NativePHP starts [Laravel queue workers](https://nativephp.com/docs/desktop/2/digging-deeper/queues) as child processes. Nativella has [one PHP thread](limitations.md#one-request-at-a-time), so a slow request holds back the others, and a queue worker needs PHP's command line on the machine.
- NativePHP [signs and notarizes](https://nativephp.com/docs/desktop/2/publishing/building) for macOS and Windows. Nativella has Debian packages, and macOS apps signed ad hoc, which Apple does not notarize. See [Distribution](limitations.md#distribution).

### On a phone

- Since version 4, NativePHP renders Blade as SwiftUI and Jetpack Compose views, without a web view (see [SuperNative](https://nativephp.com/docs/mobile/4/architecture/super-native)). The app gets the platform's scrolling and accessibility, and its screens are written for it, with EDGE components. A web view stays available, as a component. Nativella shows the pages you already have, so there is nothing to rewrite, and they remain web pages.
- NativePHP [builds with Xcode](https://nativephp.com/docs/mobile/4/getting-started/environment-setup), on a Mac with Apple silicon, and Android Studio. Bifrost does it for you, for a fee. Nativella makes an app from a [template](mobile.md#how-it-differs-from-the-desktop), without the tools of either platform.
- NativePHP [packages for the App Store and Google Play](https://nativephp.com/docs/mobile/4/publishing/introduction). Nativella writes APKs, which Google Play does not take, and no Nativella iOS app has run on a device yet.
- NativePHP's native features come as [plugins](https://nativephp.com/docs/mobile/4/plugins/introduction), Composer packages that carry Swift and Kotlin code, and you can write your own. They include push notifications. Nativella's [phone APIs](phone-apis.md) are a fixed set, in its template, without push notifications.

## Symfony UX Native

Symfony UX is a family of front-end packages. One of them compares with Nativella: UX Native, which is experimental too.

It runs nothing on the phone. It is a server-side bundle for [Hotwire Native](https://native.hotwired.dev). It detects the app's requests, gives Twig `ux_is_native()`, generates the JSON the app reads to know how to open each path, and drives its navigation from PHP. The shells are projects you create in Xcode and Android Studio, with Hotwire Native. In the words of [its documentation](https://native.hotwired.dev/overview/how-it-works): "Content is all web. Navigation is all native."

So it answers another question than Nativella:

- UX Native needs your server and the network, and every user reaches the same application and the same database. With Nativella there is no server to run. Each user has their own [database](database.md), on their machine, and the application works offline.
- With UX Native, a change to your pages reaches everyone at the next deployment, without a store review. With Nativella, the project is inside the application, so a change takes a new build.
- Hotwire Native gives native navigation, modals and tabs. Bridge components connect a page to native code, and a screen can move to Swift or Kotlin when it needs to. Nativella has a webview and [PHP APIs](platforms.md), with no Swift or Kotlin to write, and none to reach for.
- UX Native is for phones only.

### The rest of Symfony UX

Stimulus, Turbo, Twig Components, Live Components and the others build pages, and Nativella shows pages. They are not alternatives to it. No demo uses them, so none is tested. Two gaps are known. Turbo reads `response.redirected`, which stays `false` (see [What WebKit does not do](how-it-works.md#what-webkit-does-not-do)). And [nothing streams](limitations.md#the-web-layer), so what relies on server-sent events, as Mercure does, gets them all at the end. [Events](javascript.md#from-php-to-the-page-events) push to the page instead.

## Size, memory and CPU

Measured on one application: a new Laravel 13 project and its welcome page, packaged for Linux x86_64 with Nativella 0.5.0 and NativePHP Desktop 2.4.0, both with PHP 8.5. The sizes are in MB of 1,048,576 bytes, as in [Size and memory](building.md#size-and-memory).

| | Nativella | NativePHP Desktop |
|---|---|---|
| Standalone file | 94 MB, an executable | 145 MB, an AppImage |
| Debian package | 36 MB | 113 MB |
| Installed, on disk | 143 MB: the executable, and the project it extracts at first launch | 417 MB |
| Memory (PSS) | 255 MB | 405 MB |
| Memory (RSS) | 390 MB | 800 MB |
| Processes | 3 | 9, two of them PHP: the server and a queue worker, 75 MB together |
| First page, after the first launch | 0.8 s | 2 s |
| CPU at rest | 0.13% of a core | 0.43% of a core |

NativePHP carries Chromium and Node.js, and the Electron executable alone weighs 194 MB. Nativella leaves the webview to the system. WebKitGTK is not in its sizes, and takes 131 MB on a machine that does not have it yet, as a dependency of the Debian package. The desktops of Ubuntu 24.04 and later, Debian with GNOME and Fedora Workstation 43 have it already. The memory of its processes is counted.

Take the memory and the CPU as orders of magnitude, from one light page on one machine. Both ran in the same Docker container (Debian 13, WebKitGTK 2.54, Electron 40) on a virtual display, three times each, with the memory read 15 seconds after the first page and the CPU over the next 60 seconds. There is no GPU there, so GPU rendering was off in both (`WEBKIT_DISABLE_COMPOSITING_MODE=1` and `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `--disable-gpu`), and Electron ran with `--no-sandbox`. With a GPU emulated in software instead, they take 405 MB and 450 MB. Nativella's 255 MB match what its Laravel demo takes on a real display. NativePHP was not measured on one.

Under load, Nativella's PHP uses one core at most. It has [one thread](limitations.md#one-request-at-a-time), and boots the framework at each request, until its [worker mode](limitations.md#roadmap). NativePHP Mobile keeps Laravel booted between requests, by default.

On a phone, for the same project:

| APK, arm64 | Nativella | NativePHP Mobile |
|---|---|---|
| Without intl | About 36 MB less, with [your own PHP](custom-php.md#on-phones) | 53 MB |
| With intl | 85 MB | 56 MB |

The gap comes from two choices. NativePHP compresses its native libraries, which Android unpacks at install. That is 43 MB of PHP for 13 MB in the APK, so the application takes more room on the phone than the APK says. Nativella stores its library as it is, 71 MB, and Android loads it from the APK. And Nativella's ICU carries the data of every locale, 36 MB of that library, where NativePHP's ICU data take 2 MB.

NativePHP leaves intl out on Android unless you ask for it, with `native:install --with-icu`. Nativella's releases carry it, but you can customize PHP to save room. A [template with your own PHP](custom-php.md#on-phones), without intl, drops the 36 MB of ICU from every APK made from it. That size comes from the library, as this APK was not measured. It takes a build from the repository, until an application [picks its extensions](limitations.md#roadmap). Neither was measured for memory or CPU on a phone.

With Symfony UX Native, PHP takes the memory and the CPU of your server, for all your users, and the phone only runs the shell and a web view. Its apps were not measured.

## Customizing PHP

| | Nativella | NativePHP Desktop | NativePHP Mobile | Symfony UX Native |
|---|---|---|---|---|
| Version | 8.5 | 8.3, 8.4 or 8.5, from the machine that builds | 8.3, 8.4 or 8.5, from the machine that builds or from `nativephp.lock` | The one of your server |
| Extensions | [A fixed list](building.md#the-php-inside) | [A fixed list](https://github.com/NativePHP/php-bin/blob/main/php-extensions.txt), with gd, bz2 and sockets on top of Nativella's | [A fixed list](https://nativephp.com/docs/mobile/4/architecture/embedded-php), with intl optional on Android | Any |
| Another PHP | [Build `nativella`](custom-php.md) from the repository | [Your own binaries](https://nativephp.com/docs/desktop/2/digging-deeper/php-binaries), through `NATIVEPHP_PHP_BINARY_PATH`, without support | None documented | The one of your server |
| Settings | `ini_set()`, and the [`php.ini`](custom-php.md#settings) compiled into `nativella` | `ini_set()`, and `phpIni()` in `NativeAppServiceProvider` | `ini_set()` | Those of your server |

Both Nativella and NativePHP Desktop build PHP with static-php-cli, but they do not swap it the same way. NativePHP runs PHP as an executable next to the application, so another PHP is another file. Nativella links PHP into its own executable, so another PHP takes a build of `nativella`, which then serves all your applications. Choosing the extensions at `nativella build` is on the [roadmap](limitations.md#roadmap).

## Which one?

- Pick NativePHP for a Laravel application that must ship now, on Windows, on macOS or in the stores, or for native views on a phone.
- Pick Symfony UX Native for a Symfony website with accounts and shared data that wants an app on phones and stays online.
- Pick Nativella for a PHP application whose data lives on the user's machine, with any framework or none, when you want your pages as they are, a lighter application on computers, no port on the machine, and no Mac or Android Studio to build for phones. Read [Limitations and roadmap](limitations.md) first, because it is experimental and Linux is the only computer it is tested on.
