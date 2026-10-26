# Symfony

The `nativella/symfony-bundle` package runs a Symfony application in Nativella. It registers the native APIs as services, dispatches native events through the event dispatcher, migrates the database when the application starts, and adds a Nativella panel to the profiler.

The [Notes demo](../demos/symfony) is a complete Symfony application built with it.

## Installation

```bash
composer require nativella/symfony-bundle
```

It brings `nativella/nativella`, the native APIs.

> [!IMPORTANT]
> `composer require` does not enable the bundle: there is no Flex recipe yet. Add it to `config/bundles.php` yourself, or the native APIs are no services, and no native event reaches your listeners.

```php title="config/bundles.php"
return [
    // ...
    Nativella\Symfony\NativellaBundle::class => ['all' => true],
];
```

Then run the application, with the [`nativella` command](installation.md):

```bash
nativella run .
```

It needs nothing else to open in a window: the front controller of Symfony, `public/index.php`, is served as it is.

## The APIs are services

Every API class is a service, named after its class: inject `Nativella\Window`, `Nativella\Dialog`, `Nativella\Notification`… wherever you need them. `Nativella\Nativella` holds them all.

```php title="src/Controller/NoteController.php"
use Nativella\Dialog;
use Nativella\Dialog\MessageKind;
use Symfony\Component\HttpFoundation\RedirectResponse;
use Symfony\Component\Routing\Attribute\Route;

#[Route('/notes/{id}/delete', methods: ['POST'])]
public function delete(Note $note, Dialog $dialog): RedirectResponse
{
    if ($dialog->confirm(\sprintf('Delete "%s"?', $note->getTitle()), kind: MessageKind::Warning, okLabel: 'Delete')) {
        $this->notes->remove($note);
        $this->entityManager->flush();
    }

    return $this->redirectToRoute('note_index', status: 303);
}
```

## Native events are Symfony events

The bundle answers `POST /_nativella/event` before the router and the firewall run, and dispatches the event object through the event dispatcher, named after its class. A listener gets it by type:

> [!NOTE]
> Native events are handled before the firewall and the locale listeners: there is no user (`getUser()` is null), and translations use the default locale. The session is there, though (the runtime sends its cookie with native events too): read what you need from it.


```php title="src/EventListener/BootListener.php"
namespace App\EventListener;

use Nativella\Event\AppBooted;
use Nativella\GlobalShortcut;
use Nativella\Menu;
use Nativella\Menu\MenuItem;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

#[AsEventListener]
final readonly class BootListener
{
    public function __construct(
        private GlobalShortcut $globalShortcut,
        private Menu $menu,
    ) {
    }

    public function __invoke(AppBooted $event): void
    {
        $this->menu->set(
            MenuItem::submenu(
                'File',
                MenuItem::link('note.new', 'New note', '/notes/new', 'CmdOrCtrl+N'),
                MenuItem::action('note.import', 'Import…', 'CmdOrCtrl+O'),
                MenuItem::separator(),
                MenuItem::action('app.quit', 'Quit', 'CmdOrCtrl+Q'),
            ),
            MenuItem::editMenu(),
        );
        $this->globalShortcut->register('CmdOrCtrl+Shift+N');
    }
}
```

```php title="src/EventListener/MenuListener.php"
namespace App\EventListener;

use Nativella\App;
use Nativella\Event\MenuItemClicked;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

#[AsEventListener]
final readonly class MenuListener
{
    public function __construct(
        private App $app,
    ) {
    }

    public function __invoke(MenuItemClicked $event): void
    {
        if ('app.quit' === $event->id) {
            $this->app->quit();
        }
    }
}
```

Only the runtime reaches that endpoint: the bundle checks `$_SERVER['NATIVELLA_INTERNAL']`, which a page cannot set. Every event and its properties are listed in [Lifecycle and events](lifecycle.md#native-events).

## The database

Point Doctrine at the [data directory](configuration.md#the-data-directory). In `.env`:

```dotenv title=".env"
DATABASE_URL="sqlite:///${NATIVELLA_DATA_DIR:-%kernel.project_dir%/var}/app.db"
```

In Nativella, the database is `app.db` in the data directory of the application; outside it (`bin/console`, `symfony serve`, the tests), it falls back to `var/app.db`.

With `doctrine/doctrine-migrations-bundle` installed, the bundle runs the pending migrations when the application starts, on `AppBooted`, before the listeners of the application: a desktop application has no deployment step, the database of every user follows the code. Write migrations as usual, with `bin/console make:migration` or `doctrine:migrations:diff`. See [Database](database.md).

## The secret

Symfony signs and encrypts with `framework.secret`. Every user gets the same copy of `.env`, so a secret there would be shared by every installation. The runtime gives each installation its own, as `NATIVELLA_SECRET`:

```yaml title="config/packages/framework.yaml"
parameters:
    app.secret: '%env(APP_SECRET)%'

framework:
    # Each installation has its own secret in Nativella, APP_SECRET elsewhere.
    secret: '%env(default:app.secret:NATIVELLA_SECRET)%'
```

## Configuration

```yaml title="config/packages/nativella.yaml"
nativella:
    # Runs the Doctrine migrations when the application starts.
    migrate_on_boot: true

when@test:
    nativella:
        # Answers native calls with Nativella\Client\FakeClient.
        fake_client: true
```

| Option | Default | |
|---|---|---|
| `migrate_on_boot` | `true` | Runs the Doctrine migrations on `AppBooted`. Needs the DoctrineMigrationsBundle. |
| `fake_client` | `false` | Replaces the client with `Nativella\Client\FakeClient`: see [Testing](#testing). |

## Twig

| Function | |
|---|---|
| `nativella_is_native()` | Whether Nativella shows the page. Offer native features only then. |
| `nativella_is_mobile()` | Whether it runs on a phone or a tablet. |
| `nativella_platform()` | `linux`, `macos`, `windows`, `android`, `ios`, or null outside Nativella. |

```twig title="templates/note/show.html.twig"
{% if nativella_is_native() %}
    <form method="post" action="{{ path('note_export', {id: note.id}) }}">
        <button>Export…</button>
    </form>
{% endif %}
```

`url()` and `absolute_url()` work too: Symfony believes it is served from `http://localhost/`, and the runtime rewrites those URLs to the address of the window in the URL attributes of HTML and the `url()` of CSS. In JSON and scripts, use `path()`: only the requests of `fetch()`, `XMLHttpRequest` and `EventSource` to `http://localhost/` are brought back to the application. See [Pages and JavaScript](javascript.md#links-and-absolute-urls).

## The profiler

In the `dev` environment, with the WebProfilerBundle, the toolbar gets a Nativella panel: every native call of the request, with its payload, its result or its error, and its duration.

Profiles are written to disk: the values of `secrets.*` calls show as `******`.

## Testing

With `fake_client: true` in the `test` environment, the container holds a `Nativella\Client\FakeClient` instead of the real client. Tests fetch it to queue answers and check the calls:

```php title="tests/Controller/NoteControllerTest.php"
use Nativella\Client\FakeClient;
use Symfony\Bundle\FrameworkBundle\Test\WebTestCase;

final class NoteControllerTest extends WebTestCase
{
    public function testDeletingANoteAsksFirst(): void
    {
        $client = static::createClient();
        $native = static::getContainer()->get(FakeClient::class);
        $native->respond('dialog.confirm', ['confirmed' => true]);

        $client->request('POST', '/notes/1/delete');

        $this->assertResponseRedirects('/notes', 303);
        $this->assertCount(1, $native->calls('dialog.confirm'));
    }
}
```

[Testing](testing.md) explains the fake client and its default answers.

## Running in a browser

The application still runs with `symfony serve` or any web server, which is handy for working on templates. The native APIs throw a `RuntimeUnavailableException` there: guard what must keep working with `Nativella\Runtime::isNative()` in PHP, and `nativella_is_native()` in Twig.
