# Pages and JavaScript

The pages of a Nativella application are web pages: HTML, CSS and as much or as little JavaScript as you like, served by your PHP application from `php://localhost/`. This page covers how they talk to PHP, and how PHP talks back.

## From the page to PHP: requests

There is no special bridge: a page talks to PHP with links, forms and `fetch()`, as it would with any server.

```js
const response = await fetch('/notes/42/pin', { method: 'POST' });
```

The request never leaves the process, so it is cheap: a few milliseconds, most of them spent booting your framework. Native APIs are called from PHP, in the controller that answers.

Cookies and sessions work: the runtime keeps a cookie jar, since WebKit does not for a custom scheme. Redirects work too, with one difference for `fetch()`: the response of the redirect target arrives, but `response.redirected` is `false` and `response.url` is the original URL. Libraries that rely on them (Turbo does) need a hand. See [How it works](how-it-works.md#what-webkit-does-not-do).

## From PHP to the page: events

`Nativella\Page` dispatches a DOM event in the pages, named `nativella:<event>`, on `window`:

```php
$page->dispatch('note.saved', ['id' => 42, 'title' => 'Groceries']);
```

```js
window.addEventListener('nativella:note.saved', (event) => {
    console.log(event.detail.title); // "Groceries"
});
```

| Method | |
|---|---|
| `dispatch(string $event, array $detail = [], ?string $window = null): void` | Dispatches `nativella:<event>` in the page of `$window`, or of every window when null. `$detail` becomes `event.detail`. |

`$detail` is encoded as JSON: keep to what JSON can hold. The event fires on the page loaded at the time of the call: one that loads later does not get it.

This is how PHP pushes to the pages, without polling: work done in one window can refresh another, a native event handled in PHP can update the page once it is done.

```php
// The preferences window saved the theme: tell every window.
$settings->set('theme', $theme);
$page->dispatch('theme.changed', ['theme' => $theme]);
```

## Native events in the page

Every [native event](lifecycle.md#native-events) reaches the pages too, before PHP handles it, as `nativella:<name>` with its payload as `detail`:

| DOM event | `event.detail` |
|---|---|
| `nativella:app.reopened` | `{args, cwd}` |
| `nativella:window.focused`, `nativella:window.blurred`, `nativella:window.closed` | `{window}` |
| `nativella:window.themeChanged` | `{window, theme}` |
| `nativella:window.downloaded` | `{window, path}` |
| `nativella:window.dragEntered` | `{window, paths}` |
| `nativella:window.dragLeft` | `{window}` |
| `nativella:window.filesDropped` | `{window, paths}` |
| `nativella:menu.itemClicked` | `{id, checked}` |
| `nativella:tray.clicked` | `{button}` |
| `nativella:updater.installed` | `{version}` |
| `nativella:updater.failed` | `{version, message}` |
| `nativella:globalShortcut.pressed` | `{accelerator}` |

Every window gets every event: compare `detail.window` with the label of the window when it matters. A page knows its own label if PHP prints it, from `Nativella\Runtime::window()`:

```php
<body data-window="<?= htmlspecialchars(Nativella\Runtime::window() ?? '') ?>">
```

```js
const dropZone = document.querySelector('#drop-zone');
const mine = (event) => event.detail.window === document.body.dataset.window;

window.addEventListener('nativella:window.dragEntered', (event) => mine(event) && dropZone.classList.add('ready'));
window.addEventListener('nativella:window.dragLeft', (event) => mine(event) && dropZone.classList.remove('ready'));
window.addEventListener('nativella:window.filesDropped', (event) => mine(event) && dropZone.classList.replace('ready', 'busy'));
```

A menu item handled only in the page needs no PHP at all:

```js
window.addEventListener('nativella:menu.itemClicked', (event) => {
    if (event.detail.id === 'sidebar') {
        document.body.classList.toggle('without-sidebar', !event.detail.checked);
    }
});
```

## Unsaved changes

A page keeps its window open as it keeps a browser tab open, with `beforeunload`: when the window closes, a listener that cancels the event, or a `window.onbeforeunload` that returns a string, makes Nativella ask the user, "Close this window? Changes you made may not be saved.", with Close and Cancel.

```js
const form = document.querySelector('form');
let dirty = false;

form.addEventListener('input', () => (dirty = true));
form.addEventListener('submit', () => (dirty = false));
window.addEventListener('beforeunload', (event) => dirty && event.preventDefault());
```

The page is asked whatever closes the window: its close button, `Window::close()`, or the Close Window item of the menu. On Linux, the Quit item of the menu asks every window, and quits once the user confirms; `App::quit()` and a signal quit without asking. A page that does not answer within a second, a script stuck in a loop, lets its window close.

## Context menus

A right click is a page event; the menu is native. The page asks PHP for it, and PHP opens it with `Menu::popup()`: see [Menus](menus.md#context-menus).

## Links and absolute URLs

Pages live at `php://localhost/` (`http://php.localhost/` on Windows). Relative URLs, and URLs that start with `/`, work as usual.

PHP believes it is served from `http://localhost/`, so the absolute URLs it generates point there, where nothing listens. The runtime rewrites them, to the address of the window, where a page loads or links to them: the URL attributes of HTML (`href`, `src`, `action`, `formaction`, `srcset`…) and the `url()` of CSS. Links, forms, images and stylesheets built with `url()`, `route()` or `asset()` keep working. Text, form values, scripts and JSON are the user's data and stay as they are, but the requests a script sends to `http://localhost/…` with `fetch()`, `XMLHttpRequest` or `EventSource` are brought back to the application: Symfony's web debug toolbar loads itself that way. Anything else a script does with such a URL, like setting the `src` of an image, still points to where nothing listens: build URLs for JavaScript and APIs relative (`path()`, `route(absolute: false)`). A navigation to `http://localhost/…` that slipped through is still brought back to the application. Files of `public/` are served as they are.

Navigations that leave the application are handled too:

- a link to another website (`http:`, `https:`, `mailto:`, `tel:`) opens in the browser, or the mail client, of the system, instead of inside the window;
- a navigation to `http://localhost/…` that slipped through, a URL built in JavaScript for instance, is brought back to the application, with a GET.

## Developer tools

`$window->openDevtools()` opens the web inspector of a window: console, network, DOM, the usual. It only exists in debug builds of Nativella, like `cargo build -p nativella`; `cargo install` makes a release build, where it throws. See [Windows](windows.md#driving-windows).
