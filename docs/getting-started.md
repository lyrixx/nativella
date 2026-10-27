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

### Reloading

Change `index.php` and save: the window reloads. `nativella run` watches the files of the project, and reloads every window once they stop changing, with the new code: opcache checks the files on every request. It leaves out what the application and its tools write to, `var/`, `storage/`, `bootstrap/cache/`, `vendor/`, `node_modules/` and `.git/`, the data directory, the logs and SQLite databases an application writes anywhere (`*.log`, `*.sqlite`, `*.db` and their journals), and the files editors keep aside (`.index.php.swp`, `index.php~`). A [standalone executable](building.md) does not watch anything. `NATIVELLA_RELOAD=0` turns it off.

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

Step 3 only calls native code. When something must happen the other way round (the application started, a menu item was clicked, files were dropped on a window, a timer fired), PHP has to hear about it: that is a native event. An application that does not react to any can skip this step. With [Symfony](symfony.md#native-events-are-symfony-events) and [Laravel](laravel.md#native-events-are-laravel-events), a listener does what the code below does by hand.

Native events reach PHP as requests to `POST /_nativella/event`, which only the runtime can send. `EventFactory::fromGlobals()` turns such a request into an event object, and returns null for every other request. Put it at the top of the front controller:

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

`dist/hello` is one executable carrying the whole project; `dist/` itself is never packed, so building from the project directory works too. On Linux, `nativella deb hello` makes a Debian package of it. [Building a standalone app](building.md) says what goes in, and what the machine still needs.

## Where to go next

- [What runs where](platforms.md): which native API works on computers, which on phones.
- [Windows](windows.md), [Menus](menus.md), [Dialogs](dialogs.md): the native APIs, one page each.
- [Mobile](mobile.md): the same project as an Android or iOS application, with `nativella android` and `nativella ios`.
- [Database](database.md): keep data in SQLite, in the data directory of the application.
- [Pages and JavaScript](javascript.md): native events in the browser, and context menus.
- The demos of the repository: [Notes](../demos/symfony) with Symfony, [Tasks](../demos/laravel) with Laravel, [Snippets](../demos/vanilla) in plain PHP.
