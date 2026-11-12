# Plain PHP

No framework is needed. `nativella/nativella` works on its own, with one object for the APIs and one function call for the events. The [Snippets demo](../demos/vanilla) is a complete application written this way, with its own router, PHP templates and DBAL.

## Installation

```bash
composer require nativella/nativella
```

The package needs PHP 8.4 or later, on the machine that runs Composer.

## The APIs

`Nativella::create()` builds every API at once, on the client that calls the runtime:

```php
use Nativella\Nativella;

$native = Nativella::create();

$native->window->setTitle('Snippets');
$native->clipboard->writeText($snippet);
$native->notification->send('Copied', 'The snippet is in the clipboard.');
```

Its properties are `app`, `barcodeScanner`, `biometrics`, `childProcess`, `clipboard`, `dialog`, `geolocation`, `globalShortcut`, `haptics`, `menu`, `nfc`, `notification`, `page`, `power`, `screen`, `secrets`, `settings`, `share`, `shell`, `system`, `timer`, `tray`, `updater` and `window`, one per API in the [documentation](README.md#native-apis). Creating it calls nothing, so it is safe outside Nativella, until you use it.

To build the API objects one by one, give each the same client:

```php
use Nativella\Client\ExtensionClient;
use Nativella\Dialog;
use Nativella\Window;

$client = new ExtensionClient();
$dialog = new Dialog($client);
$window = new Window($client);
```

## The events

Native events arrive as requests to your front controller (`POST /_nativella/event`), sent by the runtime. `EventFactory::fromGlobals()` returns the event of such a request, and null for any other one, including a page that would post to the same path. The event is an object of its own class (`AppBooted`, `MenuItemClicked`…), or a `Nativella\Event\NativeEvent`, with its `name` and its raw `payload`, for an event this version of the package does not know. Handle it before your router:

```php title="public/index.php"
<?php

use App\NativeEventHandler;
use Nativella\Event\EventFactory;
use Nativella\Nativella;

require dirname(__DIR__).'/vendor/autoload.php';

$native = Nativella::create();

if (null !== $event = EventFactory::fromGlobals()) {
    (new NativeEventHandler($native))->handle($event);
    http_response_code(204);

    return;
}

// The router, and the pages of the application.
```

```php title="src/NativeEventHandler.php"
namespace App;

use Nativella\Event\AppBooted;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\WindowFilesDropped;
use Nativella\Menu\MenuItem;
use Nativella\Nativella;

final readonly class NativeEventHandler
{
    public function __construct(
        private Nativella $native,
    ) {
    }

    public function handle(object $event): void
    {
        match (true) {
            $event instanceof AppBooted => $this->boot(),
            $event instanceof MenuItemClicked => $this->onMenu($event->id),
            $event instanceof WindowFilesDropped => $this->native->page->dispatch('files.dropped', ['paths' => $event->paths]),
            default => null,
        };
    }

    private function boot(): void
    {
        $this->native->menu->set(
            MenuItem::submenu(
                'File',
                MenuItem::link('new', 'New snippet', '/snippets/new', 'CmdOrCtrl+N'),
                MenuItem::separator(),
                MenuItem::action('quit', 'Quit', 'CmdOrCtrl+Q'),
            ),
            MenuItem::editMenu(),
        );
    }

    private function onMenu(string $id): void
    {
        if ('quit' === $id) {
            $this->native->app->quit();
        }
    }
}
```

Every event and its properties are listed in [Lifecycle and events](lifecycle.md#native-events).

### In another framework

To dispatch native events through a framework's own dispatcher, see [Lifecycle and events](lifecycle.md#in-another-framework): a route, and `EventFactory::fromJson()` or `allFromJson()`.

## Errors

Every exception of the package implements `Nativella\Exception\ExceptionInterface`:

| Exception | Thrown when |
|---|---|
| `NativeCallException` | The native side could not do what PHP asked: the window does not exist, the platform has no such API, the system refused. Also when the payload cannot be sent: a string that is not UTF-8, `INF` or `NAN`. |
| `RuntimeUnavailableException` | PHP does not run in Nativella: in a browser through a web server, or on the command line. |
| `UnexpectedResponseException` | The runtime, or the endpoint of the [updates](updates.md), answered something the package cannot read. |
| `VersionMismatchException` | The `nativella` command and the package are not of the same release line (0.6.x with 0.6.x before 1.0, 1.x with 1.x after). It is thrown on the first native call, and says which one to upgrade. |

A `NativeCallException` has the `method` that failed (`window.open`), a `reason`, which is a sentence for people, and a `kind` to act on, a `Nativella\Exception\ErrorKind`:

| Kind | |
|---|---|
| `ErrorKind::InvalidPayload` | The payload is not what the method reads: a string that is not UTF-8, a value out of range, a path that is not absolute. |
| `ErrorKind::NotFound` | What the call names does not exist: a window, a menu item, a process, a file. |
| `ErrorKind::Timeout` | The user, or a device, did not answer in time. |
| `ErrorKind::Unavailable` | The method does not exist here: on this platform, in this build, or without the permission or the configuration it needs. |
| `ErrorKind::Failed` | The system, or a library, failed: anything the other kinds do not say. |

```php
use Nativella\Exception\ErrorKind;
use Nativella\Exception\NativeCallException;

try {
    $native->app->setBadgeCount(3);
} catch (NativeCallException $e) {
    // A platform without a badge is fine, anything else is not.
    if (ErrorKind::Unavailable !== $e->kind) {
        throw $e;
    }
}
```

## The database

Keep the database in the [data directory](configuration.md#the-data-directory), which `Runtime::dataDir()` returns, and fall back to a directory of the project outside Nativella, for `php -S`:

```php title="src/Database.php"
namespace App;

use Nativella\Runtime;

final class Database
{
    public static function connect(string $projectDir): \PDO
    {
        $dir = Runtime::dataDir() ?? $projectDir.'/var';
        if (!is_dir($dir) && !mkdir($dir, 0o777, true) && !is_dir($dir)) {
            throw new \RuntimeException(\sprintf('Cannot create the directory "%s".', $dir));
        }

        return new \PDO('sqlite:'.$dir.'/app.sqlite', options: [\PDO::ATTR_ERRMODE => \PDO::ERRMODE_EXCEPTION]);
    }
}
```

Without a framework, nothing migrates the schema for you, so do it on `AppBooted`, which the runtime handles before serving any page. [Database](database.md) shows how, with PDO or with DBAL.

## Running in a browser

The same front controller runs under `php -S localhost:8000 -t public`, which is handy to work on the pages. `EventFactory::fromGlobals()` returns null there, and the native APIs throw a `RuntimeUnavailableException`, so check `$native->isNative()` before offering a native feature.

```php
if ($native->isNative()) {
    $router->post('/snippets/{id}/copy', $snippets->copy(...));
}
```

`isNative()` asks the client, so it is also true under the [fake client](testing.md) of the tests, which then go through the native code. `Runtime::isAvailable()` only tells whether PHP runs in Nativella.

`php -S` sends no `app.booted` event either, so create the schema on the first request if it is missing.

In a browser, a page of another site can post the application's forms for the user. Give the forms that change data a CSRF token, kept in the session, and refuse a request without it. The [demo](../demos/vanilla) does it in `src/Http/CsrfToken.php`.
