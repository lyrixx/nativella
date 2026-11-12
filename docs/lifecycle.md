# Lifecycle and events

A Nativella application is one process. PHP starts once, serves every request from every window, and hears about what happens on the native side through events.

## Booting

1. Nativella reads [`extra.nativella`](configuration.md) and creates the data directory.
2. PHP starts on its own thread, once for the life of the process.
3. The runtime sends the `app.booted` event to PHP, and waits for the answer before serving anything else.
4. If no window is open by then, the main window opens, labelled `main`, with the [`window`](configuration.md#window) options in `composer.json`.

`AppBooted` is where an application gets ready: migrate the database (the [Symfony](symfony.md) and [Laravel](laravel.md) integrations do it for you), set the [menu](menus.md), create the [tray](tray.md) icon, register [global shortcuts](global-shortcuts.md), open its own windows. An application that does not handle the event answers 404, which is fine because the main window opens all the same.

## Requests

After the boot, every request from a window is a full PHP request: request startup, your front controller, request shutdown. The framework boots on every request, as under PHP-FPM; OPcache keeps the compiled code between them.

Requests are served one at a time, by the same PHP thread, whatever the window. The files in `public/` are the exception. The runtime serves them from disk without PHP, so stylesheets and images keep loading while PHP is busy.

`$_SERVER['NATIVELLA_WINDOW']` holds the label of the window that made the request, and the native APIs act on that window by default.

## Native events

When something happens on the native side, the runtime tells PHP with an internal request, `POST /_nativella/event`, whose JSON body names the event:

```json
{"name": "menu.itemClicked", "payload": {"id": "export", "checked": null}}
```

The runtime marks it with `$_SERVER['NATIVELLA_INTERNAL'] = '1'`, which a page cannot fake, because request headers only ever become `HTTP_*` variables. That is what keeps the endpoint private.

`Nativella\Event\EventFactory` turns the body into an event object:

| Event name | Class | Properties | When |
|---|---|---|---|
| `app.booted` | `AppBooted` | | PHP is ready. Handled before any page is served. |
| `app.reopened` | `AppReopened` | `args`, `cwd` | The user started the application again while it was running. |
| `app.urlsOpened` | `AppUrlsOpened` | `urls` | The system opened URLs of one of the application's [deep link schemes](configuration.md#deeplinks). Any website or application can open one, so they are user input. Check them as such. |
| `childProcess.output` | `ChildProcessOutput` | `alias`, `pid`, `stream`, `output` | A [child process](child-processes.md) wrote lines. |
| `childProcess.exited` | `ChildProcessExited` | `alias`, `pid`, `code` | A child process ended. |
| `notification.clicked` | `NotificationClicked` | `reference` | The user clicked a [notification](notifications.md#clicks) sent with a reference (Linux). |
| `power.resumed` | `PowerResumed` | | The computer woke from sleep. See [Power](power.md). |
| `power.sourceChanged` | `PowerSourceChanged` | `onBattery` | The computer went on battery, or back on mains. |
| `timer.fired` | `TimerFired` | `name` | A [timer](timers.md) fired. |
| `window.focused` | `WindowFocused` | `window` | A window got the focus. |
| `window.blurred` | `WindowBlurred` | `window` | A window lost the focus. |
| `window.closed` | `WindowClosed` | `window` | A window was closed. |
| `window.themeChanged` | `WindowThemeChanged` | `window`, `theme` | The system theme changed between light and dark. |
| `window.downloaded` | `WindowDownloaded` | `window`, `path` | A [download](downloads.md) was saved where the user chose. |
| `window.dragEntered` | `WindowDragEntered` | `window`, `paths` | Files dragged from the system entered a window. |
| `window.dragLeft` | `WindowDragLeft` | `window` | They left it without being dropped. |
| `window.filesDropped` | `WindowFilesDropped` | `window`, `paths` | Files from the system were dropped on a window. |
| `menu.itemClicked` | `MenuItemClicked` | `id`, `checked` | An item of the application menu, a context menu or the tray menu was clicked. |
| `tray.clicked` | `TrayClicked` | `button` | The tray icon was clicked. Not on Linux. |
| `updater.installed` | `UpdaterInstalled` | `version` | The next start runs this [update](updates.md). |
| `updater.failed` | `UpdaterFailed` | `version`, `message` | The update was not installed. |
| `globalShortcut.pressed` | `GlobalShortcutPressed` | `accelerator` | A [global shortcut](global-shortcuts.md) was pressed. |

All the classes live in `Nativella\Event`. `window` is a window label, `theme` a `Nativella\System\Theme`, `paths` a list of absolute paths, `path` an absolute path, `checked` the new state of a checkbox item (null for other items), `button` a `Nativella\Tray\MouseButton` (`Left`, `Middle` or `Right`).

An event this version of the package does not know arrives as a `NativeEvent`, with its `name` and its raw `payload`, so a newer runtime can add events without breaking an older application. With Symfony and Laravel, every event is also dispatched as a `NativeEvent`, after its own class, so a listener of `NativeEvent` hears them all.

## Handling events

Each integration routes the events its own way:

- With Symfony, the bundle dispatches them through the event dispatcher, named after their class. An `#[AsEventListener]` whose `__invoke()` takes a `MenuItemClicked` gets it. See [Symfony](symfony.md).
- With Laravel, the package dispatches them as Laravel events. See [Laravel](laravel.md).
- In plain PHP, `EventFactory::fromGlobals()` returns the event of the current request, as its class or as a `NativeEvent`, or null when the request is not an internal one:

```php title="public/index.php"
use Nativella\Event\EventFactory;
use Nativella\Event\WindowFilesDropped;

if (null !== $event = EventFactory::fromGlobals()) {
    if ($event instanceof WindowFilesDropped) {
        foreach ($event->paths as $path) {
            $importer->import($path);
        }
    }

    http_response_code(204);

    return;
}
```

Answer with any 2xx status. Anything else is logged by the runtime (`[event] PHP answered 500 Internal Server Error to the "menu.itemClicked" event.`), and the application goes on.

During an event there is no "current window". The native APIs act on the focused window, or on the window the event names when you pass it on:

```php
$window->setTitle('Dropped '.\count($event->paths).' files', $event->window);
```

### In another framework

An integration is a route and a few lines. Take the `POST` requests to `Nativella\Runtime::EVENT_PATH` (`/_nativella/event`) for which `Runtime::isInternalRequest()` is true, answer the others with a 404, and hand the body to `EventFactory`:

| Method | |
|---|---|
| `EventFactory::fromJson(string $body): object` | The event, as its class, or as a `NativeEvent` when the package does not know it. |
| `EventFactory::allFromJson(string $body): list<object>` | What the Symfony and Laravel integrations dispatch: the event as its class, then as a `NativeEvent`. Only the `NativeEvent` for an event the package does not know. |
| `EventFactory::create(string $name, array $payload): object` | The event of that name, from a payload already decoded. |

The first two throw a `Nativella\Exception\UnexpectedResponseException` for a body that is not an event. Answer the request with a 2xx status, 204 for instance.

## Events in the page

Every native event also reaches the JavaScript of every window, as a `nativella:<name>` DOM event on `window`, before PHP handles it:

```js
window.addEventListener('nativella:window.themeChanged', (event) => {
    document.documentElement.dataset.theme = event.detail.theme;
});
```

`app.booted` is the exception, since no page is loaded yet. PHP can send its own events to the pages too. See [Pages and JavaScript](javascript.md).

## A second launch

Only one instance of an application runs at a time. Starting it again brings the running instance to the front. It focuses one of its windows, or opens the main window again when the application only lived in the [tray](tray.md). Then it receives `AppReopened`, with the command line (`args`) and the working directory (`cwd`) of the new instance, which exits. Use it to open a file passed on the command line:

```php
use Nativella\Event\AppReopened;

if ($event instanceof AppReopened && isset($event->args[1])) {
    $native->window->navigate('/open?path='.rawurlencode($event->args[1]));
}
```

## Closing and quitting

Closing a window sends `WindowClosed`. Closing the last one quits the application, unless it has a [tray](tray.md) icon, in which case it keeps running in the tray until something calls `App::quit()`. The application decides once the `WindowClosed` listeners are done, so one that sets a tray icon, or opens a window, keeps it running.

Before a window closes, its page is asked, as a browser asks before closing a tab. A page with unsaved changes can [keep it open](javascript.md#unsaved-changes) until the user confirms.

`App::quit()` and `App::restart()` act once the current request has been answered, so the response still reaches the page. See [App](app.md).

When the user quits, PHP may be in the middle of a request, saving a form for instance. On Linux, the Quit menu item lets PHP answer the requests it already has, for 5 seconds at most, then quits. So do Ctrl+C, a closed terminal and `kill` (`SIGINT`, `SIGHUP`, `SIGTERM`) on Linux and macOS, where a second signal quits at once. The Quit menu item of macOS and Windows does not wait.
