# Updates

A [standalone executable](building.md) updates itself from the releases you publish: it asks your server whether there is a newer version, downloads it in the background, checks that you signed it, and replaces itself. The next start runs the new version.

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
| `endpoint` | The URL of the document that describes the latest release. HTTPS only; `http://localhost` and `http://127.0.0.1` too, for testing. |
| `publicKey` | The public key of the [signatures](#signing), base64-encoded as Tauri's signer prints it, or as minisign writes it. It ships with the application: it is public. |

The [`version`](configuration.md#version) of the application is what the endpoint is compared to: keep it a semantic version (`1.2.0`), and raise it with each release.

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
            "url": "https://notes.example/releases/1.3.0/notes-macos-aarch64",
            "signature": "…"
        }
    }
}
```

`version` may start with a `v`. `notes` and `pub_date` are optional. Each platform, `linux-x86_64`, `linux-aarch64`, `darwin-aarch64`, `windows-x86_64`, gives the URL of the executable of `nativella build` for it, and its signature. A platform the document leaves out gets no update.

## Signing

Every executable is signed with your secret key, which never ships, and the application checks the signature with the public key before it replaces itself: whoever controls the server, or the network, cannot make it run something you did not build.

The keys and the signatures are [minisign](https://jedisct1.github.io/minisign/)'s. With minisign:

```bash
minisign -G -p notes.pub -s notes.key           # once: keep notes.key secret, and safe
minisign -S -s notes.key -m dist/notes          # each release: writes dist/notes.minisig
base64 -w0 dist/notes.minisig                   # the "signature" of the document
base64 -w0 notes.pub                            # the "publicKey" of composer.json
```

Tauri's signer (`npx @tauri-apps/cli signer generate`, then `signer sign`) makes the same keys and signatures, already base64-encoded.

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
| `install(): ?string` | Starts installing the latest release, if it is newer: its version, null otherwise. The download goes on in the background, PHP is free meanwhile. |

Both throw a `NativeCallException` when the endpoint fails or answers a document they cannot read, and `install()` outside a standalone executable: `nativella run` runs the project as it is.

Once the executable is in place, PHP and the pages get an event:

| Event | |
|---|---|
| `UpdateInstalled` | The next start runs `$event->version`. Tell the user, or [restart](app.md) when it suits them: `$app->restart()`. |
| `UpdateFailed` | `$event->version` was not installed: `$event->message` says why, a download that failed, a signature that does not match. The executable is the same as before. |

As DOM events, `nativella:updater.installed` and `nativella:updater.failed`, with the same properties in `detail`.

## Where it works

The new executable is downloaded next to the running one, then takes its name: the running application keeps the file it started from. So the directory of the executable has to be writable by the user. An executable installed by a package manager, as the [Debian package](building.md#debian-packages) in `/usr/bin`, updates with its package manager instead: `install()` throws and says so.

Tested on Linux. macOS and Windows go through the same code, untested; on Windows, the previous executable stays next to the new one, with the `.old` extension. Phones update through their store.
