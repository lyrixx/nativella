# Getting started

From an empty directory to a desktop application, in plain PHP. With a framework the steps are the same, with less code: [Symfony](symfony.md) and [Laravel](laravel.md) say what changes.

You need the `nativella` command: see [Installation](installation.md).

## 1. A PHP application

Nativella serves anything with a `public/index.php`:

```bash
mkdir -p hello/public
cd hello
```

```php title="public/index.php"
<?php

$name = htmlspecialchars($_GET['name'] ?? 'world');
?>
<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Hello</title>
<h1>Hello, <?= $name ?>!</h1>
<form>
    <input name="name" placeholder="Your name" autofocus>
    <button>Greet me</button>
</form>
```

Run it:

```bash
nativella run .
```

A window opens on the page. Submit the form: `$_GET` is filled, as behind any web server, except that no server started. `ss -ltnp` shows no new port: the request went from the webview to PHP inside the `nativella` process.

## 2. Give it a name

The window is titled after the directory. Name the application in `composer.json`:

```json title="composer.json"
{
    "name": "acme/hello",
    "extra": {
        "nativella": {
            "name": "Hello",
            "identifier": "com.acme.hello",
            "window": { "width": 520, "height": 360, "resizable": false }
        }
    }
}
```

The identifier names the directory where the application keeps its data: pick it once. [Configuration](configuration.md) lists every key.

## 3. Call native code

Install the PHP package:

```bash
composer require nativella/nativella
```

Then call it from the page:

```php title="public/index.php"
<?php

use Nativella\Nativella;

require __DIR__.'/../vendor/autoload.php';

$native = Nativella::create();

if ('POST' === $_SERVER['REQUEST_METHOD']) {
    $native->notification->send('Hello', 'From PHP, through a desktop notification.');

    header('Location: /', true, 303);

    return;
}
?>
<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Hello</title>
<form method="post">
    <button>Notify me</button>
</form>
```

Click the button: a notification shows up, and the page comes back. The 303 redirect works although WebKit follows no redirect for a custom scheme: the runtime follows it for the page. [How it works](how-it-works.md) has the details.

## 4. React to native events

Native events (the application started, a menu item was clicked…) reach PHP as requests to `POST /_nativella/event`, which only the runtime can send. `EventFactory::fromGlobals()` turns such a request into an event object, and returns null for every other request. Put it at the top of the front controller:

```php title="public/index.php"
<?php

use Nativella\Event\AppBooted;
use Nativella\Event\EventFactory;
use Nativella\Event\MenuItemClicked;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use Nativella\Nativella;

require __DIR__.'/../vendor/autoload.php';

$native = Nativella::create();

if (null !== $event = EventFactory::fromGlobals()) {
    if ($event instanceof AppBooted) {
        $native->menu->set(
            MenuItem::submenu(
                'File',
                MenuItem::action('hello', 'Say hello', 'CmdOrCtrl+H'),
                MenuItem::separator(),
                MenuItem::role(Role::Quit),
            ),
            MenuItem::editMenu(),
        );
    } elseif ($event instanceof MenuItemClicked && 'hello' === $event->id) {
        $native->dialog->message('Hello from the menu!');
    }

    http_response_code(204);

    return;
}

// The pages of the application, as before.
```

`AppBooted` comes first, before any page is served: it is where an application sets its menus, opens its windows and migrates its database. [Lifecycle and events](lifecycle.md) lists every event.

## 5. Ship it

```bash
composer install --no-dev --optimize-autoloader
cd ..
nativella build hello
./dist/hello
```

`dist/hello` is one executable carrying the whole project. Building from the parent directory keeps `dist/` out of the project. [Building a standalone app](building.md) says what goes in, and what the machine still needs.

## Where to go next

- [Windows](windows.md), [Menus](menus.md), [Dialogs](dialogs.md): the native APIs, one page each.
- [Database](database.md): keep data in SQLite, in the data directory of the application.
- [Pages and JavaScript](javascript.md): native events in the browser, and context menus.
- The demos of the repository: [Notes](../demos/symfony) with Symfony, [Tasks](../demos/laravel) with Laravel, [Snippets](../demos/vanilla) in plain PHP.
