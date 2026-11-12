# Updates

A [standalone executable](building.md), or a [macOS app](building.md#macos-apps), updates itself from the releases you publish. It asks your server whether there is a newer version, downloads it in the background, checks that you signed it and that it is that version of the application, for this machine, and replaces itself. The next start runs the new version.

## Configuration

```json title="composer.json"
{
    "extra": {
        "nativella": {
            "version": "1.2.0",
            "updates": {
                "endpoint": "https://notes.example/releases/latest.json",
                "publicKey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6…"
            }
        }
    }
}
```

| Key | |
|---|---|
| `endpoint` | The URL of the document that describes the latest release. HTTPS only, and so are its redirections and the URLs of the executables; `http://localhost` and `http://127.0.0.1` too, for testing. |
| `publicKey` | The public key of the [signatures](#signing), base64-encoded as Tauri's signer prints it, or as minisign writes it. It is public, so it ships with the application. |

The application's [`version`](configuration.md#version) is what the endpoint is compared to. Keep it a semantic version (`1.2.0`), and raise it with each release.

## The document

The endpoint serves a JSON document in the format of [Tauri's updater](https://v2.tauri.app/plugin/updater/#static-json-file), so the tools that write it for Tauri write it for Nativella too:

```json title="latest.json"
{
    "version": "1.3.0",
    "notes": "Faster search.",
    "pub_date": "2026-10-27T10:00:00Z",
    "platforms": {
        "linux-x86_64": {
            "url": "https://notes.example/releases/1.3.0/notes-linux-x86_64",
            "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkK…"
        },
        "darwin-aarch64": {
            "url": "https://notes.example/releases/1.3.0/notes-macos-aarch64.app.tar.gz",
            "signature": "…"
        }
    }
}
```

`version` may start with a `v`. `notes` and `pub_date` are optional; `pub_date` is an RFC 3339 date, with or without a fraction of a second, or `Updater::check()` throws an `UnexpectedResponseException`. Each platform, `linux-x86_64`, `linux-aarch64`, `darwin-aarch64`, `windows-x86_64`, gives the URL of its `nativella build` executable, 1 GiB at most, or for macOS the `.tar.gz` of its app (see [Apps of macOS](#apps-of-macos)), and its signature. A platform the document leaves out gets no update.

The executable says what it is. The application keeps it only if it has its `identifier`, the `version` of the document, newer than the running one, and runs on the machine. Build each release with the `version` of composer.json raised to the one you announce, so that a release announced as `99.0.0` cannot bring back an older build, signed or not.

## Signing

Every executable is signed with your secret key, which never ships, and the application checks the signature with the public key before it replaces itself, so whoever controls the server, or the network, cannot make it run something you did not build.

The keys and the signatures are [minisign](https://jedisct1.github.io/minisign/)'s. With minisign:

```bash
minisign -G -p notes.pub -s notes.key           # once: keep notes.key secret, and safe
minisign -S -s notes.key -m dist/notes          # each release: writes dist/notes.minisig, and the same for the .tar.gz of a macOS app
base64 < dist/notes.minisig | tr -d '\n'        # the "signature" of the document
base64 < notes.pub | tr -d '\n'                 # the "publicKey" of composer.json
```

The application takes the signatures minisign 0.10 and later make by default, of a hash of the file. With an older minisign, sign with `-H`.

Tauri's signer (`npx @tauri-apps/cli signer generate`, then `signer sign`) makes the same keys and signatures, already base64-encoded.

> [!WARNING]
> An application has one public key, the one it was built with, and no way to change it. Without the secret key you cannot sign a release that the installed applications accept, so they never update again. With it, anyone who also controls the endpoint can. Keep a backup of the key and of its password, away from the server that publishes the releases.

## Checking and installing

```php
use Nativella\Updater;

$update = $updater->check();
if (null !== $update) {
    // $update->version, $update->notes, $update->date
    $updater->install();
}
```

| Method | |
|---|---|
| `check(): ?Update` | The latest release, when it is newer than this version, null otherwise. It asks the endpoint, and waits for its answer, 30 seconds at most. |
| `install(): ?string` | Starts installing the latest release, if it is newer, and returns its version, or null otherwise. The download goes on in the background, and PHP is free meanwhile. |

Once an update is being installed, or installed, both compare the endpoint to its version rather than to the running one, so they do not offer it again before the next start.

Both throw a `NativeCallException` when the application has no `updates` in its configuration, when the endpoint fails or answers a document they cannot read, and `install()` outside a standalone executable (`nativella run` runs the project as it is), or while another update is being installed. Without `updates`, and outside a standalone executable, its [kind](plain-php.md#errors) is `ErrorKind::Unavailable`.

Once the executable is in place, PHP and the pages get an event:

| Event | |
|---|---|
| `UpdaterInstalled` | The next start runs `$event->version`. Tell the user, or [restart](app.md) when it suits them: `$app->restart()`. |
| `UpdaterFailed` | `$event->version` was not installed. `$event->message` says why: a download that failed, a signature that does not match, an executable that is not the one announced or that is over 1 GiB. The executable is the same as before. |

The DOM events are `nativella:updater.installed` and `nativella:updater.failed`, with the same properties in `detail`.

## Where it works

The new executable is downloaded next to the running one, then takes its name, and the running application keeps the file it started from. So the executable's directory has to be writable by the user. An executable installed by a package manager, like the [Debian package](building.md#debian-packages) in `/usr/bin`, updates with its package manager instead, and `install()` throws and says so.

The previous executable stays next to the new one, as `<name>.old` (`notes.old`, `notes.exe.old`), until the next update. To go back to it by hand, rename it.

While it downloads, the new executable is a hidden file next to the running one, `.<name>.update-…`. An application that quits before the end leaves it there, removes it at its next start, and the next `install()` downloads from the beginning.

Tested on Linux and macOS. Windows goes through the same code, untested. On Windows, a second update in the same run fails, because the previous executable is still running: restart the application first. Phones update through their store.

## Apps of macOS

A macOS app is a directory, which the update replaces whole. Its release is the `.tar.gz` of the app that `nativella build` made, as Tauri's updater takes it, and its signature is the archive's:

```bash
COPYFILE_DISABLE=1 tar -czf notes-macos-aarch64.app.tar.gz -C dist Notes.app
minisign -S -s notes.key -m notes-macos-aarch64.app.tar.gz
```

`COPYFILE_DISABLE=1` keeps macOS's `tar` from adding the extended attributes of the files, as `._` files, which the application would ignore anyway.

The archive holds one app, of directories and files only, 1 GiB at most once extracted. The application extracts it next to itself, checks that it is the announced version of this application, for this Mac, then takes its place in one step: there is no moment without an app. The previous one stays next to the new one, as `<name>.app.old` (`Notes.app.old`), which Finder does not take for an app. The app's directory has to be writable by the user, as the Applications folder is for an administrator. An app the user did not move since they downloaded it runs from a read-only copy, and cannot update itself: ask them to move it to their Applications folder first.
