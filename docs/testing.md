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
| `dialog.confirm`, `dialog.ask` | `false` |
| `dialog.openFile`, `dialog.openDirectory`, `dialog.saveFile` | `null`: cancelled |
| `dialog.openFiles` | no files |
| `clipboard.readText` | `null` |
| `settings.has`, `settings.all` | `false`, no settings |
| `window.open`, `window.get` | a window with the label asked for, focused, 1024 by 768 |
| `window.all`, `screen.all` | none |
| `screen.primary`, `screen.cursorPosition` | none, `(0, 0)` |
| `app.info` | Nativella 0.1.0, `dev.nativella.app` |
| `app.path` | `/home/nativella/<directory>` |
| `app.isAutostartEnabled`, `globalShortcut.isRegistered` | `false` |
| `system.info`, `system.theme` | Linux, x86_64, `en-US`; `light` |

Every other method answers an empty result, which the API classes turn into nothing (`void`). `settings.get` answers no value, so `Settings::get()` returns its default.

## With a framework

The integrations replace the client in the container, so that the application under test gets a `FakeClient` wherever it injects an API:

- [Symfony](symfony.md#testing): `nativella.fake_client: true` in the `test` environment.
- [Laravel](laravel.md#testing): see the Laravel page.

## Running outside Nativella

An application is a web application first: it also runs with `symfony serve`, `php artisan serve` or `php -S`, in a browser, which is handy for working on templates. The native APIs throw there, so guard the calls that may run outside Nativella:

```php
use Nativella\Runtime;

if (Runtime::isNative()) {
    $window->setTitle($note->title);
}
```

`Runtime::isNative()` tells whether `nativella_call()` exists. In tests, prefer the fake client: it exercises the same code as in the application.
