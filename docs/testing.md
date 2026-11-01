# Testing

The native APIs only work inside Nativella: elsewhere, in PHPUnit or on the command line, `nativella_call()` does not exist. Calling an API there throws a `Nativella\Exception\RuntimeUnavailableException`. Tests swap the client that makes the calls for `Nativella\Client\FakeClient`.

## The fake client

Every API class takes a `Nativella\Client` in its constructor. `FakeClient` records the calls instead of making them, and answers what the native side would when nothing happens: no file picked, an empty clipboard, a dialog dismissed.

Take a service that exports notes where the user chooses:

```php title="src/Exporter.php"
use Nativella\Dialog;
use Nativella\Notification;

final readonly class Exporter
{
    public function __construct(
        private Dialog $dialog,
        private Notification $notification,
    ) {
    }

    /**
     * @param list<string> $notes
     */
    public function export(array $notes): void
    {
        $path = $this->dialog->saveFile('Export notes', fileName: 'notes.md');
        if (null === $path) {
            return;
        }

        file_put_contents($path, implode("\n\n", $notes));
        $this->notification->send('Exported', basename($path));
    }
}
```

Its test drives the dialog with `respond()`, and checks the notification with `calls()`:

```php title="tests/ExporterTest.php"
use Nativella\Client\FakeClient;
use Nativella\Dialog;
use Nativella\Notification;
use PHPUnit\Framework\TestCase;

final class ExporterTest extends TestCase
{
    public function testItExportsWhereTheUserChose(): void
    {
        $target = sys_get_temp_dir().'/exporter-test.md';
        $client = new FakeClient();
        $client->respond('dialog.saveFile', ['path' => $target]);

        (new Exporter(new Dialog($client), new Notification($client)))->export(['Groceries', 'Ideas']);

        $this->assertStringEqualsFile($target, "Groceries\n\nIdeas");
        $this->assertSame('Exported', $client->calls('notification.send')[0]['payload']['title']);
    }

    public function testNothingHappensWhenTheUserCancels(): void
    {
        // Unless told otherwise, dialog.saveFile answers ['path' => null].
        $client = new FakeClient();

        (new Exporter(new Dialog($client), new Notification($client)))->export(['Groceries']);

        $this->assertSame([], $client->calls('notification.send'));
    }
}
```

| Method | |
|---|---|
| `respond(string $method, array\|Throwable $result): void` | Answers the next call to `$method` with `$result`, or throws it. Call it several times to queue answers. |
| `calls(?string $method = null): list<array{method: string, payload: array}>` | The calls made so far, all of them or those to `$method`. |
| `reset(): void` | Forgets the calls and the queued answers. |

Methods are the native ones, `<domain>.<method>`: `window.open`, `dialog.confirm`, `clipboard.readText`… The payloads are the JSON objects the API classes send, with camelCase keys. The quickest way to learn them is to make the call and look at `$client->calls()`.

To test how your code handles a failure, queue an exception:

```php
use Nativella\Exception\NativeCallException;

$client->respond('globalShortcut.register', new NativeCallException('globalShortcut.register', new \RuntimeException('Already taken.')));
```

## What it answers by default

Without a queued answer, `FakeClient` returns:

| Method | Answer |
|---|---|
| `app.info` | Nativella 0.1.0, `dev.nativella.app` |
| `app.isAutostartEnabled` | `false` |
| `app.path` | `/home/nativella/<directory>` |
| `app.thirdPartyNotices` | `null`, as a development build of Nativella |
| `barcodeScanner.scan` | `null`: cancelled |
| `biometrics.authenticate`, `biometrics.isAvailable` | `false` |
| `childProcess.all` | none |
| `childProcess.start` | the process id `1` |
| `clipboard.readImage`, `clipboard.readText` | `null`: empty |
| `dialog.ask`, `dialog.confirm` | `false` |
| `dialog.openDirectory`, `dialog.openFile`, `dialog.saveFile` | `null`: cancelled |
| `dialog.openFiles` | no files |
| `geolocation.currentPosition` | `null`: not allowed |
| `globalShortcut.isRegistered` | `false` |
| `nfc.isAvailable` | `false` |
| `nfc.read` | throws a `NativeCallException`, as the runtime does when NFC is not available |
| `power.status` | on mains, without a battery |
| `screen.all`, `screen.primary` | none |
| `screen.cursorPosition` | `(0, 0)` |
| `secrets.get` | `null` |
| `settings.all`, `settings.has` | no settings, `false` |
| `settings.get` | no value: `Settings::get()` returns its default |
| `system.info`, `system.theme` | Linux, x86_64, `en-US`; `light` |
| `updater.check`, `updater.install` | `null`: this version is the latest |
| `window.all` | none |
| `window.get`, `window.open` | the window asked for, `main` by default, focused, 1024 by 768 |

The other methods return nothing (`void`): they answer an empty result.

## With a framework

The integrations replace the client in the container, so that the application under test gets a `FakeClient` wherever it injects an API:

- [Symfony](symfony.md#testing): `nativella.fake_client: true` in the `test` environment.
- [Laravel](laravel.md#testing): see the Laravel page.

## Running outside Nativella

An application is a web application first: it also runs with `symfony serve`, `php artisan serve` or `php -S`, in a browser, which is handy for working on templates. The native APIs throw there, so guard the calls that may run outside Nativella:

```php
use Nativella\Nativella;

if ($nativella->isNative()) {
    $window->setTitle($note->title);
}
```

`isNative()` of `Nativella\Nativella` (a service with Symfony, the `Nativella` facade with Laravel) asks the client: true in Nativella, and under the fake client, so that the tests go through the native code as the application does. `nativella_is_native()` in Twig and `@native` in Blade answer the same. `Runtime::isNative()` only tells whether `nativella_call()` exists, which is false in tests, fake client or not: keep it for where there is no client.
