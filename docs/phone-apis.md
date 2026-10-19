# Phone APIs

Phones can do what computers cannot: vibrate, locate themselves, recognize the fingerprint or the face of their owner. Nativella exposes these to PHP, on Android (and on iOS, untested), with Tauri's plugins.

| API | | Permission |
|---|---|---|
| `Nativella\Haptics` | Vibrations and haptic feedback. | `haptics` |
| `Nativella\Geolocation` | Where the phone is. | `geolocation` |
| `Nativella\Biometrics` | Fingerprint or face authentication. | `biometrics` |

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

On Android, every application has the plugins of the three, from the template APK, but registers only those it asks for, and keeps only the permissions they declare to Android, location for `geolocation`: an application that never locates the phone does not ask for it. iOS builds turn them into features of the `nativella` crate (`--features geolocation,haptics`): only these plugins go into the application.

Where the user must agree, they are asked on the first call, not at install: location, on Android.

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

## In the demos

- Laravel Tasks vibrates with a success when the last open task is done.
- Symfony Notes adds the location of the phone to a note, as a link to OpenStreetMap.
- Plain PHP Snippets asks for a fingerprint, rather than a dialog, before deleting a snippet.

## Not there yet

The camera, the barcode scanner, sharing, NFC and push notifications. Tauri has plugins for most of them: the barcode scanner, for one, only has a JavaScript API, which PHP cannot call.
