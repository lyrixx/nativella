# What runs where

Most native APIs are for computers, a few are for phones only, and some work on both. This page lists each one, so that you find out before you build a screen around it.

Computers are Linux, macOS and Windows: Linux is the one Nativella is tested on, the others are not (see [Limitations and roadmap](limitations.md#platforms)). Phones are Android, tested on an emulator, and iOS, in the simulator only: no phone API has been tried on iOS, and no app has run on a device (see [Mobile](mobile.md#what-is-tested)).

A call that the platform cannot answer throws a `Nativella\Exception\NativeCallException`: check `Runtime::isMobile()` first, or `nativella_is_mobile()` in Twig. A dash means that every call throws.

| API | Computers | Phones |
|---|---|---|
| [App](app.md) | Everything. | No badge count, no autostart, no `Directory::Desktop`. |
| [Windows](windows.md) | Everything. `printToPdf()` on Linux only. | Windows fill the screen: their size, position and decoration options are ignored. `center()`, `minimize()`, `maximize()`, `unmaximize()`, `setAlwaysOnTop()`, `setFullscreen()`, `setProgress()`, `print()` and `printToPdf()` throw. |
| [Menus](menus.md) | Yes. | — |
| [Tray](tray.md) | Yes. On Linux, only its menu reports clicks. | — |
| [Dialogs](dialogs.md) | Yes. | Messages, questions and `openFile()`, `openFiles()`. `openDirectory()` and `saveFile()` throw. |
| [Notifications](notifications.md) | Yes. A click reaches PHP on Linux only. | Yes, without clicks. |
| [Clipboard](clipboard.md) | Yes. | Yes. |
| [Global shortcuts](global-shortcuts.md) | Yes. On Linux, under X11 only. | — |
| [Settings](settings.md) | Yes. | Yes. |
| [Secrets](secrets.md) | Yes. | — |
| [Timers](timers.md) | Yes. | Yes. |
| [Child processes](child-processes.md) | Yes. | — |
| [Shell](shell.md) | Yes. | Everything but `trash()`. |
| [Screens](screens.md) | Yes. Under Wayland, `primary()` returns null. | Not tried. |
| [System](system.md) | Yes. | Yes. |
| [Power](power.md) | Yes. | — |
| [Pages and JavaScript](javascript.md) | Yes. | Yes. |
| [Downloads](downloads.md) | Yes. | — |
| [Phone APIs](phone-apis.md): `Haptics`, `Geolocation`, `Biometrics` | — | Android. iOS: untested. |

## What there is no API for

There is no PHP API for the camera, the microphone, sharing, NFC or push notifications, on any platform: [Phone APIs](phone-apis.md#not-there-yet) says what is missing. [Limitations and roadmap](limitations.md) lists the rest.
