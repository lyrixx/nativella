# Phone APIs

Phones can do what computers cannot: vibrate, locate themselves, recognize their owner's fingerprint or face, read a barcode with their camera or an NFC tag, share with other applications. Nativella exposes these to PHP, on Android (and on iOS, untested), with Tauri's plugins, and a community one, tauri-plugin-sharekit, for the share sheet.

| API | | Permission |
|---|---|---|
| `Nativella\Haptics` | Vibrations and haptic feedback. | `haptics` |
| `Nativella\Geolocation` | Where the phone is. | `geolocation` |
| `Nativella\Biometrics` | Fingerprint or face authentication. | `biometrics` |
| `Nativella\BarcodeScanner` | Barcodes and QR codes, read by the camera. | `barcodeScanner` |
| `Nativella\Nfc` | NFC tags, held against the phone. | `nfc` |
| `Nativella\Share` | The share sheet, with a text or a file. | None |

Two more permissions have no PHP API: `camera` and `microphone` let the [pages](#camera-and-microphone-in-the-pages) use them. `geolocation` lets the [pages](#location-in-the-pages) locate the device too.

## Permissions

An application asks for the APIs it uses in the `permissions` of its [configuration](configuration.md):

```json title="composer.json"
{
    "extra": {
        "nativella": {
            "permissions": ["geolocation", "haptics"]
        }
    }
}
```

Every application has the plugins of them all, from its platform's template, but registers only those it asks for. On Android, it keeps only the permissions they declare, location for `geolocation`, so an application that never locates the phone does not ask for it. On iOS, its `Info.plist` has the reasons only for those it asks for, which iOS shows the user: Face ID for `biometrics`, the location for `geolocation`, the camera for `barcodeScanner` and `camera`, the microphone for `microphone`, NFC for `nfc`.

Where the user must agree (location and camera, on Android), they are asked on the first call, not at install.

Calling an API that the application did not ask for throws a `NativeCallException` that says so, of the kind `ErrorKind::Unavailable`, and so does calling any of them on a computer. Only `Biometrics::isAvailable()` and `Nfc::isAvailable()` do not throw: they answer false there. Check `Runtime::isMobile()` first:

```php
use Nativella\Runtime;

if (Runtime::isMobile()) {
    $haptics->vibrate();
}
```

## Haptics

```php
use Nativella\Haptics\Feedback;
use Nativella\Haptics\Impact;

$haptics->vibrate();                     // 300 ms
$haptics->vibrate(milliseconds: 1000);
$haptics->impact(Impact::Light);         // a tap, as when an element snaps into place
$haptics->feedback(Feedback::Success);   // the pattern of a success, a warning or an error
$haptics->selection();                   // a tick, as when a selection changes
```

| Method | |
|---|---|
| `vibrate(int $milliseconds = 300): void` | Vibrates. |
| `impact(Impact $impact = Impact::Medium): void` | A tap: `Light`, `Medium`, `Heavy`, `Soft` or `Rigid`. |
| `feedback(Feedback $feedback): void` | `Success`, `Warning` or `Error`. |
| `selection(): void` | A tick. |

Android plays all of them with its vibrator; iOS has a haptic engine for the last three.

## Geolocation

```php
use Nativella\Geolocation\Accuracy;

$position = $geolocation->currentPosition();
if (null === $position) {
    // The user did not allow it.
}

$position->latitude;  // 45.76730
$position->longitude; // 4.83433
$position->accuracy;  // 20.0: the phone is within 20 meters
```

`currentPosition(Accuracy $accuracy = Accuracy::Balanced, int $maximumAge = 0): ?Position` asks the user the first time, then blocks until the phone knows where it is. It returns null when the user does not allow it. It throws a `NativeCallException` when the location is turned off on the phone, when the user leaves the permission question unanswered for 60 seconds, or when the phone does not find its position within 20 seconds. Indoors, GPS may never get a fix.

| Argument | |
|---|---|
| `$accuracy` | `Accuracy::Balanced`: Wi-Fi and cell towers, fast and frugal, to within some tens of meters. `Accuracy::High`: GPS if there is one, a few meters, at the cost of time and battery. |
| `$maximumAge` | In seconds: how old a position the phone already knows may be. 0 asks for a new one. |

A `Position` has `latitude` and `longitude` (degrees), `accuracy` (meters), `altitude` (meters), `altitudeAccuracy` (meters), `speed` (meters per second), `heading` (degrees, clockwise from the north) and `timestamp` (a `DateTimeImmutable`). The last four are null when the phone does not know.

On Android 12 and later, the user may grant an approximate location only, and the position is then about a kilometer off, whatever the accuracy asked.

## Biometrics

```php
if (!$biometrics->authenticate('Show the private notes')) {
    return $this->redirectToRoute('note_home');
}
```

| Method | |
|---|---|
| `isAvailable(): bool` | Whether the phone has a fingerprint or a face enrolled. False on a computer, and when the application does not ask for `biometrics`. |
| `authenticate(string $reason, ?string $title = null): bool` | Asks the user to prove who they are, and blocks until they do. `$reason` tells them why. False when they cancel, or do not match. Throws a `NativeCallException` when the phone has no lock at all. |

The user can fall back on the phone's PIN, as everywhere else on it, and a phone without a fingerprint or a face enrolled asks for the PIN only. So call `authenticate()` whatever `isAvailable()` says, because skipping it when biometrics are not available would let anyone holding a phone locked by a PIN through. Biometrics only prove that the phone's owner holds it. There is no key or identity to check on a server.

## Barcode scanner

```php
use Nativella\BarcodeScanner\Format;

$barcode = $barcodeScanner->scan([Format::Ean13, Format::QrCode]);
if (null === $barcode) {
    // The user did not allow the camera, or cancelled.
}

$barcode->content; // "3017620422003"
$barcode->format;  // Format::Ean13
```

`scan(array $formats = [], string $cancelLabel = 'Cancel'): ?Barcode` asks the user the first time, then shows the camera and blocks until it reads a code. It returns null when the user does not allow the camera, or cancels. It throws a `NativeCallException` when the camera fails, when the user leaves the permission question unanswered for 60 seconds, or when it reads no code within 2 minutes.

| Argument | |
|---|---|
| `$formats` | The kinds of codes to read: `Format::Aztec`, `Codabar`, `Code128`, `Code39`, `Code93`, `DataMatrix`, `Ean13`, `Ean8`, `Itf`, `Pdf417`, `QrCode`, `UpcA`, `UpcE`. Every kind when empty. |
| `$cancelLabel` | The label of the cancel button, in the application's language. |

The camera shows behind the page. While PHP waits, Nativella makes the page transparent and hides it, and draws a frame and the cancel button on top. The page comes back as it was. A `Barcode` has the `content` of the code, and its `format`, null for a kind of code that `Format` does not list.

On Android, ML Kit reads the codes with a model that ships in the APK, without Google Play services or a download. It always reads QR codes, whatever the `$formats`. `Codabar` and `UpcA` are Android's. iOS reads a UPC-A code as an EAN-13 one, with a leading zero, and fails when asked for either.

The call blocks PHP, so make it in a request the page waits for, a link or a form, and redirect once it is done. A `fetch()` gets a 504 after 25 seconds on Android (see [Mobile](mobile.md#limitations)).

## NFC

```php
$tag = $nfc->read();

$tag->id;                    // "04A1B2C3D4E5F6"
$tag->records[0]->uri();     // "https://notes.example/42"
$tag->records[1]->text();    // "Shelf B"
```

| Method | |
|---|---|
| `isAvailable(): bool` | Whether the phone has NFC, turned on in its settings. False on a computer, and when the application does not ask for `nfc`. |
| `read(): Tag` | Waits for a tag to come near the phone, and reads it. Throws a `NativeCallException` when NFC is not available, or when no tag comes within 60 seconds. |

A `Tag` has its `id` in hexadecimal, null when the phone does not tell it, its `technologies` (Android's classes, as `android.nfc.tech.NfcA`, or iOS's families, as `MiFare`) and its `records`, the NDEF message it holds, empty when it holds none. A `Record` has its `format` (a `Nfc\TypeNameFormat`: `Empty`, `WellKnown`, `Media`, `AbsoluteUri`, `External`, `Unknown` or `Unchanged`), and its `type`, `id` and `payload` as bytes; `text()` and `uri()` read the text and URI records, which most tags hold, and return null for the others.

The call blocks PHP, so make it in a request the page waits for, and tell the user to hold the tag against the phone first, because Android shows nothing while it waits. A `fetch()` gets a 504 after 25 seconds on Android (see [Mobile](mobile.md#limitations)).

NFC has not been tried on a phone, as emulators have no NFC (`isAvailable()` is false there). On iOS, reading tags needs the NFC Tag Reading entitlement. `nativella ios` signs an app with the entitlements of its provisioning profile, so turn that capability on for the App ID of the profile (see [Mobile](mobile.md#for-devices)). This is untested. Writing tags is not there yet.

## Share

```php
$share->text("{$note->getTitle()}\n\n{$note->getBody()}");

$share->file(Runtime::dataDir().'/export.csv', 'text/csv');
```

| Method | |
|---|---|
| `text(string $text): void` | Opens the share sheet with a text or a link. The application the user picks makes what it can of it. |
| `file(string $path, ?string $mimeType = null, ?string $title = null): void` | Opens it with a file, absolute and readable by PHP, such as one in the data directory or the cache. `$mimeType` (`text/csv`, `image/png`…) tells the phone which applications can take it. `$title` is the title of the sheet, on Android. Throws when there is no such file. |

Sharing needs no permission, because the user picks where it goes, or closes the sheet. The call returns once the sheet is open, and nothing tells whether the user shared, as Android does not say. The file is copied to the application's cache first, so PHP can delete or change its own afterwards.

## Camera and microphone in the pages

The application's pages may use the camera and the microphone themselves, with `navigator.mediaDevices.getUserMedia()`, for a video call, a photo or a voice note. The application asks for them in its `permissions`, `camera` and `microphone`, one by one:

```js
const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
document.querySelector('video').srcObject = stream;
```

A page that asks for what the application did not ask for gets a `NotAllowedError`. The user is asked on the first use, as in a browser: by Android for the camera and the microphone, by iOS with the reason in the `Info.plist`. Computers ask no question. Linux's webview lets the page use the devices once the application asked for them, and this has not been tried on macOS or Windows.

These two are not phone APIs. They work on computers too, and need no PHP.

## Location in the pages

`geolocation` lets the pages locate the device themselves, with `navigator.geolocation`, on computers too:

```js
navigator.geolocation.getCurrentPosition(
    (position) => showOnMap(position.coords.latitude, position.coords.longitude),
    (error) => console.warn(error.message),
);
```

Without `geolocation` in the `permissions`, the page gets a `PERMISSION_DENIED` error. With it, the system has the last word. Linux's webview asks GeoClue, the desktop's location service, which fails with `POSITION_UNAVAILABLE` when the location services are off in the settings. It has not been tried on Android, iOS, macOS or Windows yet.

## In the demos

- Laravel Tasks vibrates with a success when the last open task is done.
- Symfony Notes adds the phone's location to a note, as a link to OpenStreetMap.
- Symfony Notes adds what a barcode or a QR code holds to a note, with a link for a web address.
- Symfony Notes shares a note, its title and its text, with the share sheet.
- Plain PHP Snippets asks the user to prove who they are before deleting a snippet, with `authenticate()` whatever `isAvailable()` says, and falls back on a dialog only when the phone has no lock.

## Not there yet

PHP has no API yet for the camera (photos and videos) or the microphone, for writing NFC tags, or for push notifications. Tauri has no plugin for the first two, but the pages can use them with `getUserMedia()` (see above).
