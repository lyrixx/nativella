# Mobile

Nativella builds Android and iOS applications from the same PHP project as the desktop one, with PHP inside. The three demos — [Symfony](../demos/symfony), [Laravel](../demos/laravel) and [plain PHP](../demos/vanilla) — run on Android: forms, sessions and CSRF tokens, the SQLite database and its migrations, native dialogs, the file picker and the clipboard. On iOS, the Symfony demo runs in the simulator.

> [!WARNING]
> Mobile support is experimental: Android (arm64, API level 28 or later) is tested on an emulator, iOS in the simulator, neither on a real phone. See [what is tested](#what-is-tested).

## How it differs from the desktop

On a phone there is no `nativella` executable to append the project to: the application is a library that Android or iOS loads. So:

- On Android, the library is built once per release, without any project, into a template APK: `nativella android` adds the project to it, packed the same way `nativella build` does, as an asset of the APK. On iOS, `nativella bundle` packs it into a file that the build compiles **into** the library (the `NATIVELLA_BUNDLE` variable). Either way, the application extracts it once per build, in its cache directory, at first launch.
- PHP is linked statically: phones have no PHP to share. The build of the library downloads a static `libphp.a` built for the phone, with its headers (see [PHP for the phone](#php-for-the-phone)).
- Pages are served from `http://php.localhost/`, as Tauri does for custom schemes on Android. PHP sees that origin too.
- Android's WebView drops the body of the requests it hands to the application: Nativella injects a script that sends the body of forms, `fetch()` and `XMLHttpRequest` to the runtime first, through Tauri's IPC. Synchronous `XMLHttpRequest` bodies cannot be captured.
- What PHP writes to stderr goes to logcat, under the `nativella` tag.

## In PHP

`Nativella\Runtime::isMobile()` tells a phone from a computer, `Runtime::platform()` returns `"android"` or `"ios"`. The Symfony bundle has the same in Twig: `nativella_is_mobile()` and `nativella_platform()`.

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

`nativella android` makes the APK of a project from the template APK of its release, `nativella-android-template-arm64.apk`, without any Android tool: no JDK, no Android SDK, no Rust. Take both from the same [release](https://github.com/lyrixx/nativella/releases), and [prepare the project](building.md#preparing-the-project) as for a standalone executable:

```bash
nativella android path/to/project --template nativella-android-template-arm64.apk -o dist/notes.apk
adb install dist/notes.apk
```

> [!NOTE]
> The command and the template must come from the same version of Nativella: `nativella android` warns when they do not. Both come with Nativella 0.2.0 and later.

The APK is the template, with:

- the bundle of the project, as the asset `nativella.bundle`, which the application reads in place at startup: what [ships](building.md#what-ships) in a standalone executable, the local overrides left out and listed;
- the identity of the project, from the `extra.nativella` section of its `composer.json`, in its manifest: its application ID, what Android knows the application by, made of the `identifier`; the `name` under the icon; the `version` as the version name, and as the version code, `major × 1,000,000 + minor × 1,000 + patch`, which Android compares to accept an update;
- the Android permissions of the [phone APIs](phone-apis.md) the application asks for (`permissions`), and of them only: the template has them all, and the application only registers the ones it asks for;
- the [`icon`](configuration.md#icon) of the project as its launcher icon, or the Nativella icon without one: drawn for each screen density, from 48 to 192 pixels, inside a margin, its corners rounded, as `tauri icon` draws the launcher icons of Android.

It is aligned, then signed with APK Signature Scheme v2, which Android checks from API level 24, by `--key`: an EC P-256 private key, in PKCS #8 PEM.

```bash
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out notes-key.pem
nativella android path/to/project --template nativella-android-template-arm64.apk --key notes-key.pem
```

Without `--key`, `nativella` signs with the debug key of the computer, which it makes at its first use, in the configuration directory of the user (`~/.config/nativella/android-debug-key.pem` on Linux): enough to try the application, not to publish it. Android installs an update only if it is signed with the same key as the installed application: keep yours.

The application ID is the `identifier` with its dashes as underscores, as Android accepts no dash: `dev.nativella.note-taker` becomes `dev.nativella.note_taker`. Android also wants two segments at least, each starting with a letter: `nativella android` refuses an identifier that cannot make one, such as `com.acme.2048`, and says what to put in `extra.nativella.identifier`.

The classes of the template keep their package, `dev.nativella.runtime`, which the library names in its JNI functions: only the application ID changes, as when the `applicationId` of a Gradle build differs from its `namespace`.

### From the repository

`castor mobile:android symfony` does the same for a demo, with a template built from the repository. It runs in Docker, in the `android` image of [the stack](development.md): the builder, plus a JDK, the Android SDK with an NDK, the Android target of Rust and the Tauri command. Docker is all it needs on your machine. The image is x86_64 only, as Google ships no NDK for Linux on ARM: on an ARM machine, Docker has to emulate it.

```bash
castor mobile:android symfony
```

The task builds the `android` image first, a few gigabytes, once. Then it:

1. builds the template, as `castor mobile:android-template` does: it generates the Android project of `crates/nativella/tauri.conf.json` in `crates/nativella/gen/android`, runs `cargo tauri android build` with every phone API, and copies the APK to `target/nativella/android-template-debug.apk`;
2. copies the demo to `target/nativella/stage/`, without `vendor/`, `var/`, `tests/` and the local `.env` files, and prepares it for production there: Composer without the dev dependencies, the production environment (`composer dump-env prod` for Symfony, `APP_ENV=production` and `APP_DEBUG=false` for Laravel);
3. makes its APK with `nativella android`, in the builder, which has no Android tool: `target/nativella/android/symfony.apk`.

`--release` makes it from a release template instead, `target/nativella/android-template-release.apk`, whose library has no debug symbols: the one a release publishes (`castor mobile:android-template --release`).

```bash
castor mobile:android-emulator symfony
```

installs the APK afresh, without the data of a previous install, on the emulator, or the phone, that `adb` sees (from `ANDROID_HOME`, or `PATH`), opens it, and waits for PHP to answer a page: it fails if PHP answers none within 2 minutes, or if the app stops. The screenshot and what the app wrote to logcat land in `target/nativella/`.

Without Docker, `castor --context=host mobile:android symfony` runs the same build on your machine, with its tools: what the desktop build needs, from [Developing Nativella](development.md#on-your-machine); PHP and Composer; the Android SDK with an NDK (`ANDROID_HOME`, the most recent NDK unless `NDK_HOME` says otherwise); a JDK 17 or later (`JAVA_HOME`); `rustup target add aarch64-linux-android` and `cargo install tauri-cli --version '^2' --locked`. The task says what is missing before it starts.

## Building an iOS app

`castor mobile:ios symfony` builds an app for the iOS simulator (arm64, unsigned), on a Mac with Xcode, [XcodeGen](https://github.com/yonaskolb/XcodeGen), `rustup target add aarch64-apple-ios-sim` and the Tauri command. It stages and bundles the demo in Docker; `--bundle` takes a bundle made beforehand instead, by `castor nativella:bundle symfony`, as the CI does for its macOS runners, which have no Docker. It is experimental: the [Mobile workflow](development.md#continuous-integration) runs it, then `castor mobile:ios-simulator symfony`, which opens the app in a simulator, as `mobile:android-emulator` does on Android: it fails if PHP answers no page, and leaves the screenshot and the output of the app in `target/nativella/`. There, Tauri sends what the app writes to the log of the system, in debug builds only: the task reads it with `log show`.

## What is tested

| | The CI | By hand |
|---|---|---|
| Android | Builds the template APK (arm64) and makes the APK of the Symfony demo of it with `nativella android`, on every change to the crates, the packages, the Symfony demo or the Docker stack, and opens it in an Android 16 emulator: PHP must answer a page. The APK, the screenshot and the logcat output are artifacts of the run. | The three demos, on an Android 16 emulator (x86_64, which runs arm64 through its ARM translation): pages, forms, the database, dialogs, the file picker, the clipboard and the [phone APIs](phone-apis.md). Their three APKs, made from one template, installed side by side. |
| iOS | Builds the Symfony demo for the simulator (arm64) on a macOS runner, and opens it in a simulator: PHP must answer a page. The screenshot and the output of the app are artifacts of the run. | Nothing more: no device, no phone API. |

PHP answers a page when the runtime writes `[php] GET <url> 200`: the framework booted, migrated its database and rendered the page. The [Mobile workflow](development.md#continuous-integration) runs these jobs.

## PHP for the phone

The build of the library downloads the static PHP builds that [NativePHP](https://github.com/NativePHP) publishes for its own mobile runtime (MIT, built with static-php-cli and the NDK) into `mobile/php/dist/`, until Nativella builds its own:

- PHP 8.5, thread safe, arm64 only: x86_64 emulators run it through their ARM translation;
- the extensions a Symfony or Laravel application needs: ctype, curl, dom, fileinfo, filter, iconv, mbstring, opcache, openssl, pdo_sqlite, session, sodium, sqlite3, tokenizer, xml, zlib… but not intl, which [the desktop PHP](building.md#the-php-inside) has;
- NativePHP's own extension, whose callbacks into its host Nativella answers with "nothing available" (`crates/runtime/src/nativephp_stubs.rs`).

ext-php-rs reads `php -i` and `php-config` to generate its bindings, which the PHP of the computer cannot answer for the phone: `mobile/php/php` and `mobile/php/php-config` stand in for them, from the headers of the downloaded build.

## Limitations

- **First launch is slow**: the project is extracted, then the framework builds its cache on the phone (about 5 seconds for the Symfony demo on an emulator). The following requests take a few hundred milliseconds in a debug build.
- **Debug templates are big** (about 220 MB), and so are the APKs made of them: the library is not stripped. A release template weighs about 36 MB, the APK of the Symfony demo made of it 40 MB.
- **25 seconds per page**: Android's webview, through wry, gives up on a page after 30 seconds and shows an error in its place. So when PHP takes longer, because the user left a dialog, the file picker or a fingerprint prompt open, Nativella answers first with a page that waits: it loads the same page again for a `GET`, the page of the form for a `POST`, once PHP is done. The response PHP gives at last is dropped, so a redirect after a form is lost, not the form's work. A `fetch()` or an XHR gets a 504 instead, even one that asks for HTML, as Turbo does.
- **No adaptive icon**: the launcher icon is a plain one, which launchers that shape icons, such as the Pixel one, shrink onto a white shape. No round icon, nor a monochrome one for themed icons.
- **No Google Play bundle**: `nativella android` writes APKs, to install with `adb` or to open on the phone; Google Play asks for an Android App Bundle (AAB), which a Gradle build, in a Docker image, will make. Its keys are EC P-256 ones, in PEM files, not Java keystores yet.
- **Few phone APIs**: [haptics, geolocation and biometrics](phone-apis.md). No camera, sharing or push notifications yet.
- **iOS** is experimental: [`castor mobile:ios`](#building-an-ios-app) builds for the simulator only. There, the Symfony demo shows its first page, but the webview leaves the bottom of the screen black. What the app writes reaches the log of the system in debug builds only. No phone API, nor any device, has been tried.
