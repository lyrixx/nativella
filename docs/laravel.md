# Laravel

The `nativella/laravel` package runs a Laravel application in Nativella. It binds the native APIs in the container and gives them facades, dispatches native events as Laravel events, keeps the SQLite database in the application's data directory, migrates it when the application starts, and encrypts with a secret of its own for each installation.

The [Tasks demo](../demos/laravel) is a complete Laravel application built with it.

## Installation

```bash
composer require nativella/laravel
```

It brings `nativella/nativella`, the native APIs. The package needs PHP 8.4 or later, and Laravel 12 or 13. Package discovery registers `Nativella\Laravel\NativellaServiceProvider`. Then run the application, with the [`nativella` command](installation.md):

```bash
nativella run .
```

## The APIs, injected or through facades

Every API class is a singleton in the container. Type-hint `Nativella\Window`, `Nativella\Dialog`, `Nativella\Notification`… in a controller, a listener or a job. Each one also has a facade in `Nativella\Laravel\Facades`:

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

The package routes `POST /_nativella/event` outside the `web` middleware group, and dispatches the event object through the event dispatcher. A listener of a native event has no session and no authenticated user, so `session()` and `auth()->user()` give nothing there. Listeners in `app/Listeners` are discovered by the type of their `handle()` method, as usual:

```php title="app/Listeners/SetApplicationMenu.php"
namespace App\Listeners;

use Illuminate\Support\Facades\Log;
use Nativella\Event\AppBooted;
use Nativella\Exception\NativeCallException;
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

        try {
            GlobalShortcut::register('CmdOrCtrl+Alt+Shift+T');
        } catch (NativeCallException $e) {
            // Another application holds it: the application works without.
            Log::warning('The global shortcut is not available.', ['exception' => $e]);
        }
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

The route answers 404 to anything but the runtime. It checks `$_SERVER['NATIVELLA_INTERNAL']`, which a page cannot set. Every event and its properties are listed in [Lifecycle and events](lifecycle.md#native-events).

After its own class, every event is also dispatched as a `Nativella\Event\NativeEvent`, with its `name` and its raw `payload`. Listen to it for all the events, or for one this version of the package has no class for.

A native call that fails throws a `NativeCallException`, which says why with a `kind`. See [Errors](plain-php.md#errors).

## The database

In Nativella, the package points the `sqlite` connection at `database.sqlite` in the [data directory](configuration.md#the-data-directory), creates the file, and makes `sqlite` the default connection, whatever `.env` says: it ignores `DB_URL`, and the failed jobs and the job batches of the queue go to that connection too. Outside Nativella (`php artisan`, `php artisan serve`, the tests), your database configuration applies unchanged.

When the application starts, on `AppBooted`, the package runs `migrate --force` before the application's listeners, so every user's database follows the code, with no deployment step. Write migrations as usual. On a new database, a schema dump of `php artisan schema:dump` (`database/schema/sqlite-schema.sql`) is loaded first, through PDO, because users' machines have no `sqlite3` command. See [Database](database.md).

To work on the application's database from the command line, give Artisan the same data directory:

```bash
NATIVELLA_DATA_DIR=~/.local/share/com.acme.tasks php artisan tinker
```

## The encryption key

The `.env` file ships with the application, and its `APP_KEY` with it, so every user would have the same key and anyone holding the application could forge any installation's cookies. In Nativella, the package replaces `app.key` with the secret the runtime creates for each installation (`NATIVELLA_SECRET`), and empties `app.previous_keys`. There is nothing to configure.

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

`@unlessnative … @endnative` shows it only outside Nativella:

```blade
<form method="post" action="{{ route('tasks.destroy', $task) }}" @unlessnative onsubmit="return confirm('Delete this task?')" @endnative>
```

The `Nativella\Laravel\Http\Middleware\EnsureNative` middleware hides a route outside Nativella, with a 404:

```php title="routes/web.php"
use App\Http\Controllers\ExportTasksController;
use Nativella\Laravel\Http\Middleware\EnsureNative;

Route::post('/tasks/export', ExportTasksController::class)->name('tasks.export')->middleware(EnsureNative::class);
```

Laravel builds absolute URLs (`route()`, `url()`, `asset()`) from the request, so they point to `http://localhost/…` in Nativella. The runtime rewrites them to the window's address in the HTML URL attributes and CSS `url()` that PHP answers, and links, forms and assets work as usual. In JSON and scripts, use relative URLs (`route('notes.show', $note, absolute: false)`), because only `fetch()`, `XMLHttpRequest` and `EventSource` requests to `http://localhost/` are brought back to the application. See [Pages and JavaScript](javascript.md#links-and-absolute-urls).

In a [standalone build](building.md#what-ships), the `public/storage` link of `php artisan storage:link` ships as an empty directory, so the files of the public disk are not served through it. Serve them from a route instead.

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

The application still runs with `php artisan serve`, which is handy for working on views. The native APIs throw a `RuntimeUnavailableException` there, so guard what must keep working with `Nativella::isNative()`, `@native` or the `EnsureNative` middleware.
