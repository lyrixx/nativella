# Mobile

Nativella builds Android applications from the same PHP project as the desktop one. The three demos — [Symfony](../demos/symfony), [Laravel](../demos/laravel) and [plain PHP](../demos/vanilla) — run on Android: forms, sessions and CSRF tokens, the SQLite database and its migrations, native dialogs, the file picker and the clipboard.

> [!WARNING]
> Mobile support is experimental. It is built and tested on Android (arm64, API level 28 or later). On iOS, the Symfony demo opens in the simulator, no more.

## How it differs from the desktop

On a phone there is no `nativella` executable to append the project to: the application is a library that Android loads. So:

- `nativella bundle` packs the project, the same way `nativella build` does, into a file that the build compiles **into** the library (the `NATIVELLA_BUNDLE` variable). The application extracts it once per build, in its cache directory, at first launch.
- PHP is linked statically: phones have no PHP to share. The build downloads a static `libphp.a` built for the phone, with its headers (see [PHP for the phone](#php-for-the-phone)).
- Pages are served from `http://php.localhost/`, as Tauri does for custom schemes on Android. PHP sees that origin too.
- Android's WebView drops the body of the requests it hands to the application: Nativella injects a script that sends the body of forms, `fetch()` and `XMLHttpRequest` to the runtime first, through Tauri's IPC. Synchronous `XMLHttpRequest` bodies cannot be captured.
- What PHP writes to stderr goes to logcat, under the `nativella` tag.

## In PHP

`Nativella\Runtime::isMobile()` tells a phone from a computer, `Runtime::platform()` returns `"android"`. The Symfony bundle has the same in Twig: `nativella_is_mobile()` and `nativella_platform()`.

A phone has no menu bar, tray icon, global shortcut, autostart or badge: `Menu`, `Tray`, `GlobalShortcut`, `App::setBadgeCount()` and the autostart methods of `App` throw a `NativeCallException` there, so check first:

```php
use Nativella\Runtime;

if (!Runtime::isMobile()) {
    $menu->set(/* … */);
    $tray->set(/* … */);
}
$window->open('main');
```

Windows fill the screen: their size, position and decoration options are ignored, and `Window::center()`, `minimize()`, `maximize()`, `unmaximize()`, `setAlwaysOnTop()`, `setFullscreen()`, `setProgress()` and `print()` throw. So do `Dialog::openDirectory()`, `Dialog::saveFile()`, `Shell::trash()`, `Secrets`, `ChildProcess`, `Power` and `App::path(Directory::Desktop)`.

`Dialog::openFile()` and `openFiles()` open the file picker of the system: it answers `content://` URIs, which PHP cannot read, so Nativella copies the picked files into the cache directory of the application and hands PHP their path there. Message dialogs, notifications, the clipboard, settings, timers, the shell, the system information and the page events work as on the desktop.

Phones have APIs of their own: [haptics, geolocation and biometrics](phone-apis.md), which an application asks for in its `permissions`.

Pages are drawn under the status and navigation bars: add `viewport-fit=cover` to the viewport and pad the page with `env(safe-area-inset-*)`, as the Symfony demo does.

## Building an APK

The build is a [Castor](https://castor.jolicode.com) task of the repository, written for its projects: the demos, whose path repositories point to `packages/`. It runs in Docker, in the `android` image of [the stack](development.md): the builder, plus a JDK, the Android SDK with an NDK, the Android target of Rust and the Tauri command. Docker is all it needs on your machine. The image is x86_64 only, as Google ships no NDK for Linux on ARM: on an ARM machine, Docker has to emulate it.

```bash
castor mobile:android symfony
```

The task builds the `android` image first, a few gigabytes, once. Then it:

1. copies the demo to `target/nativella/stage/`, without `vendor/`, `var/`, `tests/` and the local `.env` files, and prepares it for production there: Composer without the dev dependencies, the production environment (`composer dump-env prod` for Symfony, `APP_ENV=production` and `APP_DEBUG=false` for Laravel);
2. bundles it with `nativella bundle`, into `target/nativella/app.bundle`;
3. generates the Android project in `crates/nativella/gen/android` for the identifier of the application (the `extra.nativella` section of its `composer.json`), and runs `cargo tauri android build`, with the features of the [phone APIs](phone-apis.md) it asks for.

It prints the path of the APK. Install it on a phone or an emulator with `adb install`. `--release` builds a release APK, which still has to be signed.

Without Docker, `castor --context=host mobile:android symfony` runs the same build on your machine, with its tools: what the desktop build needs, from [Installation](installation.md); PHP and Composer; the Android SDK with an NDK (`ANDROID_HOME`, the most recent NDK unless `NDK_HOME` says otherwise); a JDK 17 or later (`JAVA_HOME`); `rustup target add aarch64-linux-android` and `cargo install tauri-cli --version '^2' --locked`. The task says what is missing before it starts.

### iOS

`castor mobile:ios symfony` builds an app for the iOS simulator (arm64, unsigned), on a Mac with Xcode, [XcodeGen](https://github.com/yonaskolb/XcodeGen), `rustup target add aarch64-apple-ios-sim` and the Tauri command. It stages and bundles the demo in Docker; `--bundle` takes a bundle made beforehand instead, as the CI does on its macOS runners, which have no Docker. It is experimental: the [Mobile workflow](development.md#continuous-integration) runs it, then `castor mobile:ios-simulator symfony`, which opens the app in a simulator and takes a screenshot once it had 30 seconds to start.

## PHP for the phone

The build downloads the static PHP builds that [NativePHP](https://github.com/NativePHP) publishes for its own mobile runtime (MIT, built with static-php-cli and the NDK) into `mobile/php/dist/`, until Nativella builds its own:

- PHP 8.5, thread safe, arm64 only: x86_64 emulators run it through their ARM translation;
- the extensions a Symfony or Laravel application needs: ctype, curl, dom, fileinfo, filter, iconv, mbstring, opcache, openssl, pdo_sqlite, session, sodium, sqlite3, tokenizer, xml, zlib… but not intl;
- NativePHP's own extension, whose callbacks into its host Nativella answers with "nothing available" (`crates/runtime/src/nativephp_stubs.rs`).

ext-php-rs reads `php -i` and `php-config` to generate its bindings, which the PHP of the computer cannot answer for the phone: `mobile/php/php` and `mobile/php/php-config` stand in for them, from the headers of the downloaded build.

## Limitations

- **First launch is slow**: the project is extracted, then the framework builds its cache on the phone (about 5 seconds for the Symfony demo on an emulator). The following requests take a few hundred milliseconds in a debug build.
- **Debug APKs are big** (about 220 MB): the library is not stripped. Release builds are much smaller.
- **25 seconds per page**: Android's webview, through wry, gives up on a page after 30 seconds and shows an error in its place. So when PHP takes longer, because the user left a dialog, the file picker or a fingerprint prompt open, Nativella answers first with a page that waits: it loads the same page again for a `GET`, the page of the form for a `POST`, once PHP is done. The response PHP gives at last is dropped, so a redirect after a form is lost, not the form's work. A `fetch()` or an XHR gets a 504 instead, even one that asks for HTML, as Turbo does.
- **One identifier at a time**: the Android project in `crates/nativella/gen/android` is generated for one application; the build generates it again when the identifier changes.
- **Few phone APIs**: [haptics, geolocation and biometrics](phone-apis.md). No camera, sharing or push notifications yet.
- **iOS** is experimental: [`castor mobile:ios`](#ios) builds for the simulator only. There, the Symfony demo shows its first page, but the webview leaves the bottom of the screen black, and what the app writes does not reach the output files of `castor mobile:ios-simulator`. No phone API, nor any device, has been tried.
