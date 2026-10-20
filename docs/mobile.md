# Mobile

Nativella builds Android and iOS applications from the same PHP project as the desktop one, with PHP inside, without the tools of their platform. The three demos — [Symfony](../demos/symfony), [Laravel](../demos/laravel) and [plain PHP](../demos/vanilla) — run on Android: forms, sessions and CSRF tokens, the SQLite database and its migrations, native dialogs, the file picker and the clipboard. On iOS, the Symfony demo runs in the simulator.

> [!WARNING]
> Mobile support is experimental: Android (arm64, and x86_64 for emulators, API level 28 or later) is tested on an emulator, iOS in the simulator, neither on a real phone: no iOS device has run an app of Nativella yet. See [what is tested](#what-is-tested).

## How it differs from the desktop

On a phone there is no `nativella` executable to append the project to: the application is a library that Android or iOS loads. So:

- The library is built once per release, without any project, into templates: a template APK for Android, template apps for iOS. `nativella android` and `nativella ios` add the project to one, packed the same way `nativella build` does: an asset of the APK, a file of the app. The application extracts it once per build, in its cache directory, at first launch.
- PHP is linked statically: phones have no PHP to share. The build of the library links the PHP of the desktop, built for the phone as a static `libphp.a`, with its libraries and its headers (see [PHP for the phone](#php-for-the-phone)).
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

### Licenses

As a [standalone executable](building.md#licenses), an APK contains the software of others, under their licenses: the PHP of the phone and its libraries, the Rust crates of the library, and the Java and Kotlin libraries of the Android project of Tauri (AndroidX, Kotlin, Google Play services for geolocation, under Google's terms). The template carries their third-party notices, and so does every APK made of it: the asset `THIRD_PARTY_NOTICES.txt`, which [`App::thirdPartyNotices()`](app.md#methods) returns, for an "about" page, since an application has no other place to show them. The releases attach them too, `THIRD_PARTY_NOTICES-android.txt`. Add the licenses of what your project ships, as on the desktop.

The Rust crates and the Java libraries come from the build of the template: cargo-about, and the dependencies Gradle resolves, with the license their POM declares. PHP and its libraries come from [the build of PHP for the phone](#php-for-the-phone): static-php-cli dumps the licenses of their sources, as for the desktop, with the ones of what php-src bundles. Like the desktop one, that PHP links [LGPL](building.md#licenses) code statically: libbcmath and libmbfl, inside php-src, and GNU libiconv. The notices give the version of each and the URL of its source archive, and `nativella android` packs an application again.

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

Without Docker, `castor --context=host mobile:android symfony` runs the same build on your machine, with its tools: what the desktop build needs, from [Developing Nativella](development.md#on-your-machine); PHP and Composer; the Android SDK with an NDK (`ANDROID_HOME`, the most recent NDK unless `NDK_HOME` says otherwise); a JDK 17 or later (`JAVA_HOME`); `rustup target add aarch64-linux-android` (`x86_64-linux-android` for `--abi=x86_64`) and `cargo install tauri-cli --version '^2' --locked`; for PHP, a C and C++ compiler for your machine, which builds the tools of ICU, CMake, make, Perl and pkg-config. The task says what is missing before it starts.

## Building an iOS app

`nativella ios` makes the iOS app of a project from a template app of its release, without Xcode nor a Mac. A release has two templates, both for arm64:

- `nativella-ios-template-iphonesimulator-arm64.zip`, for the iOS simulator of a Mac with Apple silicon;
- `nativella-ios-template-iphoneos-arm64.zip`, for iPhones and iPads.

Take the command and the template from the same [release](https://github.com/lyrixx/nativella/releases), and [prepare the project](building.md#preparing-the-project) as for a standalone executable.

> [!WARNING]
> iOS support is experimental: the CI makes the app of the Symfony demo for the simulator, and opens it there. No app made for devices has run on one yet. See [what is tested](#what-is-tested).

> [!NOTE]
> The command and the template must come from the same version of Nativella: `nativella ios` warns when they do not. Both come with Nativella 0.3.0 and later.

### For the simulator

```bash
nativella ios path/to/project --template nativella-ios-template-iphonesimulator-arm64.zip -o dist/notes.app
xcrun simctl install booted dist/notes.app
xcrun simctl launch booted dev.nativella.notes
```

The app is the app of the template, with:

- the bundle of the project, as the file `nativella.bundle`, next to its executable, which the app reads in place at startup: what [ships](building.md#what-ships) in a standalone executable, the local overrides left out and listed;
- the identity of the project, from the `extra.nativella` section of its `composer.json`, in its `Info.plist`: the `identifier` as its bundle ID, as it is, since Apple allows letters, digits, dashes and dots there; the `name` under the icon; the `version`, which iOS compares to accept an update, as three integers, without its pre-release or build suffix: `1.2.0-beta.1` becomes `1.2.0`;
- the reasons iOS gives the user when the app uses a [phone API](phone-apis.md) it asks for (`permissions`), and for them only: Face ID for `biometrics`, the location for `geolocation`. The template has the plugins of them all, and the app only registers the ones it asks for.

It is signed ad hoc: the simulator of a Mac with Apple silicon runs no code that is not signed, and the signature binds the files of the app, as Xcode's "Sign to Run Locally" does. The [Mobile workflow](development.md#continuous-integration) checks it with `codesign --verify` before it opens the app in a simulator.

### For devices

An iPhone runs the apps a developer that Apple knows signed, with a provisioning profile that lets them run. It takes a membership of the [Apple Developer Program](https://developer.apple.com/programs/), and on its website:

1. a certificate: Apple Development, to run the app on your devices, Apple Distribution for the App Store. Without a Mac, make its private key and its signing request with OpenSSL, upload the request, download the certificate, and put both in a PKCS #12 file:

    ```bash
    openssl req -new -newkey rsa:2048 -nodes -keyout ios-key.pem -out ios.csr -subj "/CN=Jane Doe/emailAddress=jane@example.com"
    # Certificates, +, Apple Development: upload ios.csr, download development.cer
    openssl x509 -inform DER -in development.cer -out development.pem
    openssl pkcs12 -export -inkey ios-key.pem -in development.pem -out ios.p12
    ```

    On a Mac, export the certificate with its key from Keychain Access, as a `.p12` file;

2. an App ID: the `identifier` of the project, or a wildcard one, such as `dev.nativella.*`;

3. for development, the devices, by their UDID;

4. a provisioning profile, of the App ID, with the certificate and, for development, the devices: a `.mobileprovision` file.

```bash
export NATIVELLA_CERTIFICATE_PASSWORD='the password of ios.p12'
nativella ios path/to/project --template nativella-ios-template-iphoneos-arm64.zip \
    --certificate ios.p12 --profile notes.mobileprovision -o dist/notes.ipa
```

The password of the certificate comes from the variable `NATIVELLA_CERTIFICATE_PASSWORD`, not from an argument, which the other users of the computer see. The file may be encrypted as OpenSSL 3 does (AES), or as Keychain Access and `openssl pkcs12 -legacy` do (3DES and RC2).

`nativella ios` checks first that the profile is for iOS, has not expired, lists the certificate, and is for the bundle ID of the app. The IPA is the app, as for the simulator, with:

- the profile, as `embedded.mobileprovision`;
- a signature as Xcode makes it: by the certificate, with the chain of Apple's certificates that issued it, which `nativella` carries; with the entitlements of the profile, its application identifier made for the bundle ID of the app when the App ID is a wildcard one.

Install it on a device of the profile with the Devices and Simulators window of Xcode, Apple Configurator or, on Linux, `ideviceinstaller` of [libimobiledevice](https://libimobiledevice.org/); upload it signed for distribution to App Store Connect, for TestFlight and the App Store, with Transporter.

> [!WARNING]
> Untested: no IPA made by `nativella ios` has been installed on a device yet. Its designated requirement and its entitlements are the bytes Xcode writes, checked against an app Xcode signed, its CMS signature verifies with OpenSSL, and the signature of its executable with [rcodesign](https://github.com/indygreg/apple-platform-rs); but only a device checks the chain of certificates, the profile and the entitlements. Tell what a device says in an [issue](https://github.com/lyrixx/nativella/issues).

### Licenses of iOS apps

As an APK, an iOS app contains the software of others, under their licenses: the PHP of the phone and its libraries, and the Rust crates of the library, with the Swift code of Tauri and of its plugins, which comes with their crates. The library of the template carries their third-party notices, as the `nativella` command carries its own, and so does every app made of it: [`App::thirdPartyNotices()`](app.md#methods) returns them, for an "about" page. The releases attach them too, `THIRD_PARTY_NOTICES-ios-iphonesimulator.txt` and `THIRD_PARTY_NOTICES-ios-iphoneos.txt`. As on Android, they give the version and the source of the [LGPL](building.md#licenses) code that PHP links, and `nativella ios` packs an app again. Add the licenses of what your project ships.

### From the repository, on a Mac

`castor mobile:ios symfony` does the same for a demo, for the simulator, with a template built from the repository, on a Mac with Xcode, [XcodeGen](https://github.com/yonaskolb/XcodeGen), `rustup target add aarch64-apple-ios-sim` and the Tauri command (`cargo install tauri-cli --version '^2' --locked`). It:

1. builds the template, as `castor mobile:ios-template` does: it builds [PHP for the simulator](#php-for-the-phone) first, gathers the third-party notices, generates the Xcode project of `crates/nativella/tauri.conf.json` in `crates/nativella/gen/apple`, runs `cargo tauri ios build --no-sign` with every phone API, and zips the app in `target/nativella/ios-template-iphonesimulator-debug.zip`. `mobile:ios-template --device` builds the template of devices instead, with `rustup target add aarch64-apple-ios`, and `--release` one without debug symbols, as a release does;
2. stages the demo, as `castor mobile:android` does;
3. makes its app with `nativella ios`, in the builder: `target/nativella/ios/symfony.app`.

`--template` takes a template for the simulator built beforehand, by a Mac, a release or the CI: the task then runs wherever Docker does, as the [Mobile workflow](development.md#continuous-integration) does on Linux, of the template its macOS runner built.

```bash
castor mobile:ios-simulator symfony
```

checks the signature of the app with `codesign --verify`, installs it in a simulator, the newest iPhone of the newest iOS of Xcode, opens it, and waits for PHP to answer a page, as `mobile:android-emulator` does: it fails if PHP answers none within 2 minutes, or if the app stops, and leaves the screenshot and the output of the app in `target/nativella/`. Tauri sends what the app writes to the log of the system, in debug builds only: the task reads it with `log show`.

## What is tested

| | The CI | By hand |
|---|---|---|
| Android | Builds the template APK (arm64) and makes the APK of the Symfony demo of it with `nativella android`, on every change to the crates, the packages, the Symfony demo or the Docker stack, and opens it in an Android 16 emulator: PHP must answer a page. The APK, the screenshot and the logcat output are artifacts of the run. | The three demos, on an Android 16 emulator (x86_64, which runs arm64 through its ARM translation): pages, forms, the database, dialogs, the file picker, the clipboard and the [phone APIs](phone-apis.md). Their three APKs, made from one template, installed side by side. The Symfony demo built for x86_64, which the emulator runs natively. |
| iOS | Builds PHP for iOS and its simulator, then the template app for the simulator (arm64), on a macOS runner; makes the app of the Symfony demo of it with `nativella ios` on Linux; then, on a macOS runner, checks its ad hoc signature with `codesign --verify` and opens it in a simulator: PHP must answer a page. The app, the screenshot and the output of the app are artifacts of the run. The signing for devices is checked by unit tests, with certificates and a profile made for them, which Apple did not issue. | Nothing more: no device, no phone API. The signature for devices was checked against an app Xcode signed, with OpenSSL and rcodesign. |

PHP answers a page when the runtime writes `[php] GET <url> 200`: the framework booted, migrated its database and rendered the page. The [Mobile workflow](development.md#continuous-integration) runs these jobs.

## PHP for the phone

The PHP of a phone is the one of [the desktop](building.md#the-php-inside), built for the phone by Nativella: the same sources, the same libraries, the same extensions, from the same list. The build of the template builds it first, once (`castor static-php:android`, see [Developing Nativella](development.md#php-for-the-phones)):

- PHP 8.5, without thread safety, as on the desktop: it runs on a thread of its own, one request at a time (see [How it works](how-it-works.md#one-php-thread));
- the extensions a Symfony, Laravel or Doctrine application needs, intl included: bcmath, ctype, curl, dom, fileinfo, filter, iconv, intl, mbstring, opcache, openssl, pdo_sqlite, phar, session, simplexml, sodium, sqlite3, tokenizer, xml, xmlreader, xmlwriter, zip, zlib;
- for Android 9 (API level 28) and later, arm64; and x86_64, for the emulators of x86_64 computers, which then run it without their ARM translation: `castor mobile:android symfony --abi=x86_64`. The releases publish the arm64 template only;
- without JIT, of PCRE or of opcache: iOS runs no code it did not sign, and on Android, the JIT of PCRE needs a function of the runtime of clang that Rust does not link. PCRE interprets the regular expressions instead;
- intl carries ICU and its data, for every locale: about 36 MB of the library, as of the executables of the desktop, which the APK stores uncompressed, as Android loads the library from there. A release template weighs about 75 MB, its library 72 MB, twice the 37 MB it weighed with NativePHP's PHP, which had no intl.

ICU is C++: the application library carries the C++ library of the NDK (`c++_static`), as no other code of the application needs one to share.

ext-php-rs reads `php -i` and `php-config` to generate its bindings, which the PHP of the computer cannot answer for the phone: the build of PHP has its own `php-config`, and `mobile/php/php` stands in for `php -i`, from its headers.

On iOS, `castor static-php:ios` builds the same PHP on a Mac, with Xcode, for iOS 15 and later, for the device and the simulator, and `castor mobile:ios-template` links it into the template apps: the Mobile workflow builds both on a macOS runner, and the template of the simulator, then opens an app made of it in the simulator. No device has run it.

## Limitations

- **First launch is slow**: the project is extracted, then the framework builds its cache on the phone (about 5 seconds for the Symfony demo on an emulator). The following requests take a few hundred milliseconds in a debug build.
- **Debug templates are big** (about 280 MB), and so are the APKs made of them: the library is not stripped. A release template weighs about 75 MB, the APK of the Symfony demo made of it 79 MB: ICU and its data, for intl, are half of it (see [PHP for the phone](#php-for-the-phone)).
- **25 seconds per page**: Android's webview, through wry, gives up on a page after 30 seconds and shows an error in its place. So when PHP takes longer, because the user left a dialog, the file picker or a fingerprint prompt open, Nativella answers first with a page that waits: it loads the same page again for a `GET`, the page of the form for a `POST`, once PHP is done. The response PHP gives at last is dropped, so a redirect after a form is lost, not the form's work. A `fetch()` or an XHR gets a 504 instead, even one that asks for HTML, as Turbo does.
- **No adaptive icon**: the launcher icon is a plain one, which launchers that shape icons, such as the Pixel one, shrink onto a white shape. No round icon, nor a monochrome one for themed icons.
- **The icon of iOS apps is the template's**: the `icon` of `extra.nativella` does not reach the iOS app yet.
- **No Google Play bundle**: `nativella android` writes APKs, to install with `adb` or to open on the phone; Google Play asks for an Android App Bundle (AAB), which a Gradle build, in a Docker image, will make. Its keys are EC P-256 ones, in PEM files, not Java keystores yet.
- **Few phone APIs**: [haptics, geolocation and biometrics](phone-apis.md). No camera, sharing or push notifications yet.
- **iOS** is experimental: in the simulator, the Symfony demo shows its first page, but the webview leaves the bottom of the screen black, and what the app writes reaches the log of the system in debug builds only. No device has run an [app for devices](#for-devices), nor has any phone API been tried.
- **iOS apps get every entitlement of their profile**, as re-signing tools give them, not the ones the app needs: those of the capabilities of its App ID, as the Apple Developer website lists them.
- **iOS apps link every phone API**: App Store Connect may refuse an app whose code can use the location or Face ID without the reason in its `Info.plist` (ITMS-90683), which only the APIs it asks for have.
- **No nested code in iOS apps**: `nativella ios` signs the executable and seals the other files; frameworks and app extensions, which the template has none of, would need signing of their own.
