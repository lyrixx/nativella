# Laravel

The `nativella/laravel` package runs a Laravel application in Nativella. It binds the native APIs in the container and gives them facades, dispatches native events as Laravel events, keeps the SQLite database in the data directory of the application, migrates it when the application starts, and encrypts with a secret of its own for each installation.

The [Tasks demo](../demos/laravel) is a complete Laravel application built with it.

## Installation

With the path repository of [Installation](installation.md#the-php-package) in your `composer.json`:

```bash
composer require nativella/nativella:^0.1@dev nativella/laravel:^0.1@dev
```

Package discovery registers `Nativella\Laravel\NativellaServiceProvider`. Then run the application:

```bash
nativella run .
```

## The APIs, injected or through facades

Every API class is a singleton of the container: type-hint `Nativella\Window`, `Nativella\Dialog`, `Nativella\Notification`… in a controller, a listener or a job. Each one also has a facade in `Nativella\Laravel\Facades`:

```php title="app/Http/Controllers/TaskController.php"
use App\Models\Task;
use Illuminate\Http\RedirectResponse;
use Nativella\Dialog\MessageKind;
use Nativella\Laravel\Facades\Dialog;

public function destroy(Task $task): RedirectResponse
{
    if (Dialog::confirm("Delete \"{$task->title}\"?", kind: MessageKind::Warning, okLabel: 'Delete')) {
        $task->delete();
    }

    return redirect()->route('tasks.index');
}
```

Some facades share their name with Laravel's: `Nativella\Laravel\Facades\App` and `Notification` are not `Illuminate\Support\Facades\App` and `Notification`. Import the one you mean, or alias it.

## Native events are Laravel events

The package routes `POST /_nativella/event` outside the `web` middleware group (no session, no CSRF token), and dispatches the event object through the event dispatcher. Listeners in `app/Listeners` are discovered by the type of their `handle()` method, as usual:

```php title="app/Listeners/SetApplicationMenu.php"
namespace App\Listeners;

use Nativella\Event\AppBooted;
use Nativella\Laravel\Facades\GlobalShortcut;
use Nativella\Laravel\Facades\Menu;
use Nativella\Menu\MenuItem;

final class SetApplicationMenu
{
    public function handle(AppBooted $event): void
    {
        Menu::set(
            MenuItem::submenu(
                'File',
                MenuItem::link('tasks.new', 'New task', '/', 'CmdOrCtrl+N'),
                MenuItem::action('tasks.export', 'Export…', 'CmdOrCtrl+E'),
                MenuItem::separator(),
                MenuItem::action('app.quit', 'Quit', 'CmdOrCtrl+Q'),
            ),
            MenuItem::editMenu(),
        );
        GlobalShortcut::register('CmdOrCtrl+Alt+Shift+T');
    }
}
```

```php title="app/Listeners/RunMenuAction.php"
namespace App\Listeners;

use Nativella\App;
use Nativella\Event\MenuItemClicked;

final readonly class RunMenuAction
{
    public function __construct(
        private App $app,
    ) {
    }

    public function handle(MenuItemClicked $event): void
    {
        if ('app.quit' === $event->id) {
            $this->app->quit();
        }
    }
}
```

The route answers 404 to anything but the runtime: it checks `$_SERVER['NATIVELLA_INTERNAL']`, which a page cannot set. Every event and its properties are listed in [Lifecycle and events](lifecycle.md#native-events).

## The database

In Nativella, the package points the `sqlite` connection at `database.sqlite` in the [data directory](configuration.md#the-data-directory), creates the file, and makes `sqlite` the default connection, whatever `.env` says. Outside Nativella (`php artisan`, `php artisan serve`, the tests), your database configuration applies unchanged.

When the application starts, on `AppBooted`, the package runs `migrate --force` before the listeners of the application: the database of every user follows the code, with no deployment step. Write migrations as usual. See [Database](database.md).

To work on the database of the application from the command line, give Artisan the same data directory:

```bash
NATIVELLA_DATA_DIR=~/.local/share/com.acme.tasks php artisan tinker
```

## The encryption key

The `.env` file ships with the application, and its `APP_KEY` with it: the same key for every user, and anyone holding the application could forge the cookies of any installation. In Nativella, the package replaces `app.key` with the secret the runtime creates for each installation (`NATIVELLA_SECRET`). There is nothing to configure.

## Configuration

Publish the configuration to change it:

```bash
php artisan vendor:publish --tag=nativella-config
```

| Key | Default | |
|---|---|---|
| `nativella.database` | `true` | In Nativella, keeps the `sqlite` connection in the data directory and makes it the default. |
| `nativella.migrate_on_boot` | `true` | Runs `migrate --force` on `AppBooted`. |

## Blade, routes and URLs

`@native … @endnative` shows a part of a view only in Nativella:

```blade title="resources/views/tasks/index.blade.php"
@native
    <form method="post" action="{{ route('tasks.export') }}">
        @csrf
        <button>Export…</button>
    </form>
@endnative
```

The `Nativella\Laravel\Http\Middleware\EnsureNative` middleware hides a route outside Nativella, with a 404:

```php title="routes/web.php"
use Nativella\Laravel\Http\Middleware\EnsureNative;

Route::post('/tasks/export', ExportTasks::class)->name('tasks.export')->middleware(EnsureNative::class);
```

Laravel builds absolute URLs (`route()`, `url()`, `asset()`) from the request, so `http://localhost/…` in Nativella. The runtime rewrites them to the address of the window in the URL attributes of HTML and the `url()` of CSS that PHP answers: links, forms and assets work as usual. In JSON and scripts, use relative URLs (`route('notes.show', $note, absolute: false)`): only the requests of `fetch()`, `XMLHttpRequest` and `EventSource` to `http://localhost/` are brought back to the application. See [Pages and JavaScript](javascript.md#links-and-absolute-urls).

## Testing

`Nativella::fake()` swaps the client for a `Nativella\Client\FakeClient`, in the container and behind every facade, and returns it. `Nativella::isNative()` and `@native` then behave as in Nativella:

```php title="tests/Feature/CopyTasksTest.php"
use Nativella\Laravel\Facades\Nativella;

public function testTheTasksAreCopied(): void
{
    $native = Nativella::fake();

    $this->post('/tasks/copy')->assertRedirect('/');

    $this->assertCount(1, $native->calls('clipboard.writeText'));
}
```

To test a native event, dispatch it as Laravel would: `event(new AppBooted())`. [Testing](testing.md) explains the fake client and its default answers.

## Running in a browser

The application still runs with `php artisan serve`, which is handy for working on views. The native APIs throw a `RuntimeUnavailableException` there: guard what must keep working with `Nativella::isNative()`, `@native` or the `EnsureNative` middleware.
