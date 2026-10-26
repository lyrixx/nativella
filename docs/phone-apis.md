# Phone APIs

Phones can do what computers cannot: vibrate, locate themselves, recognize the fingerprint or the face of their owner, read a barcode with their camera. Nativella exposes these to PHP, on Android (and on iOS, untested), with Tauri's plugins.

| API | | Permission |
|---|---|---|
| `Nativella\Haptics` | Vibrations and haptic feedback. | `haptics` |
| `Nativella\Geolocation` | Where the phone is. | `geolocation` |
| `Nativella\Biometrics` | Fingerprint or face authentication. | `biometrics` |
| `Nativella\BarcodeScanner` | Barcodes and QR codes, read by the camera. | `barcodeScanner` |

Two more permissions have no API of PHP: `camera` and `microphone` let the [pages](#camera-and-microphone-in-the-pages) use them.

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

Every application has the plugins of the four, from the template of its platform, but registers only those it asks for. On Android, it keeps only the permissions they declare, location for `geolocation`: an application that never locates the phone does not ask for it. On iOS, its `Info.plist` has the reasons of those it asks for only, which iOS shows the user: Face ID for `biometrics`, the location for `geolocation`, the camera for `barcodeScanner` and `camera`, the microphone for `microphone`.

Where the user must agree, they are asked on the first call, not at install: location and camera, on Android.

Calling an API that the application did not ask for throws a `NativeCallException` that says so, and so does calling any of them on a computer. Check `Runtime::isMobile()` first:

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

`currentPosition(Accuracy $accuracy = Accuracy::Balanced, int $maximumAge = 0): ?Position` asks the user the first time, then blocks until the phone knows where it is. It returns null when the user does not allow it. It throws a `NativeCallException` when the location is turned off on the phone, or when the phone does not find its position within 20 seconds: indoors, GPS may never get a fix.

| Argument | |
|---|---|
| `$accuracy` | `Accuracy::Balanced`: Wi-Fi and cell towers, fast and frugal, to within some tens of meters. `Accuracy::High`: GPS if there is one, a few meters, at the cost of time and battery. |
| `$maximumAge` | In seconds: how old a position the phone already knows may be. 0 asks for a new one. |

A `Position` has `latitude` and `longitude` (degrees), `accuracy` (meters), `altitude` (meters), `altitudeAccuracy` (meters), `speed` (meters per second), `heading` (degrees, clockwise from the north) and `timestamp` (a `DateTimeImmutable`). The last four are null when the phone does not know.

On Android 12 and later, the user may grant an approximate location only: the position is then about a kilometer off, whatever the accuracy asked.

## Biometrics

```php
if ($biometrics->isAvailable() && !$biometrics->authenticate('Show the private notes')) {
    return $this->redirectToRoute('note_home');
}
```

| Method | |
|---|---|
| `isAvailable(): bool` | Whether the phone has a fingerprint or a face enrolled. |
| `authenticate(string $reason, ?string $title = null): bool` | Asks the user to prove who they are, and blocks until they do. `$reason` tells them why. False when they cancel, or do not match. |

The user can fall back on the PIN of the phone, as everywhere else on it. Biometrics prove that the owner of the phone holds it, nothing more: there is no key or identity to check on a server.

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

`scan(array $formats = [], string $cancelLabel = 'Cancel'): ?Barcode` asks the user the first time, then shows the camera and blocks until it reads a code. It returns null when the user does not allow the camera, or cancels. It throws a `NativeCallException` when the camera fails, or when it reads no code within 2 minutes.

| Argument | |
|---|---|
| `$formats` | The kinds of codes to read: `Format::Aztec`, `Codabar`, `Code128`, `Code39`, `Code93`, `DataMatrix`, `Ean13`, `Ean8`, `Itf`, `Pdf417`, `QrCode`, `UpcA`, `UpcE`. Every kind when empty. |
| `$cancelLabel` | The label of the button that gives up, in the language of the application. |

The camera shows behind the page: while PHP waits, Nativella makes the page transparent and hides it, and draws a frame and the button to cancel on top. The page comes back as it was. A `Barcode` has the `content` of the code, and its `format`, null for a kind of code that `Format` does not list.

On Android, ML Kit reads the codes with a model that ships in the APK: no Google Play services, no download. It always reads QR codes, whatever the `$formats`. `Codabar` and `UpcA` are Android's: iOS reads a UPC-A code as an EAN-13 one, with a leading zero, and fails when asked for either.

The call blocks PHP, so make it in a request the page waits for, a link or a form, and redirect once it is done. A `fetch()` gets a 504 after 25 seconds on Android: see [Mobile](mobile.md#limitations).

## Camera and microphone in the pages

The pages of the application may use the camera and the microphone themselves, with `navigator.mediaDevices.getUserMedia()`: a video call, a photo, a voice note. The application asks for them in its `permissions`, `camera` and `microphone`, one by one:

```js
const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
document.querySelector('video').srcObject = stream;
```

A page that asks for what the application did not ask for gets a `NotAllowedError`. The user is asked on the first use, as in a browser: by Android for the camera and the microphone, by iOS with the reason of the `Info.plist`. Computers have no question: Linux's webview lets the page use the devices once the application asked for them, and nothing tried it on macOS or Windows.

These two are not phone APIs: they work on computers too, and need no PHP.

## In the demos

- Laravel Tasks vibrates with a success when the last open task is done.
- Symfony Notes adds the location of the phone to a note, as a link to OpenStreetMap.
- Symfony Notes adds what a barcode or a QR code holds to a note: a link for a web address.
- Plain PHP Snippets asks for a fingerprint, rather than a dialog, before deleting a snippet.

## Not there yet

A PHP API for the camera (photos and videos) and the microphone, sharing, NFC and push notifications. Tauri has no plugin for the first two: the pages can use them with `getUserMedia()`, see above.
