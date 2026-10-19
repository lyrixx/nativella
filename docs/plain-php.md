# Plain PHP

No framework is needed. `nativella/nativella` works on its own: one object for the APIs, one function call for the events. The [Snippets demo](../demos/vanilla) is a complete application written this way, with a hand-written router, PHP templates and DBAL.

## Installation

```bash
composer require nativella/nativella
```

## The APIs

`Nativella::create()` builds every API at once, on the client that calls the runtime:

```php
use Nativella\Nativella;

$native = Nativella::create();

$native->window->setTitle('Snippets');
$native->clipboard->writeText($snippet);
$native->notification->send('Copied', 'The snippet is in the clipboard.');
```

Its properties are `app`, `biometrics`, `childProcess`, `clipboard`, `dialog`, `geolocation`, `globalShortcut`, `haptics`, `menu`, `notification`, `page`, `power`, `screen`, `secrets`, `settings`, `shell`, `system`, `timer`, `tray` and `window`, one per API of the [documentation](README.md#native-apis). Creating it calls nothing: it is safe outside Nativella, until you use it.

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

Native events arrive as requests to your front controller: `POST /_nativella/event`, sent by the runtime. `EventFactory::fromGlobals()` returns the event of such a request, and null for any other one, including a page that would post to the same path. Handle it before your router:

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

Without a framework, nothing migrates the schema for you: do it on `AppBooted`, which the runtime handles before serving any page. [Database](database.md) shows how, with PDO or with DBAL.

## Running in a browser

The same front controller runs under `php -S localhost:8000 -t public`, which is handy to work on the pages. `EventFactory::fromGlobals()` returns null there, and the native APIs throw a `RuntimeUnavailableException`: check `Runtime::isNative()` before offering a native feature.

```php
use Nativella\Runtime;

if (Runtime::isNative()) {
    $router->post('/snippets/{id}/copy', $snippets->copy(...));
}
```

`php -S` sends no `app.booted` event either: create the schema on the first request if it is missing.
