# Notifications

`Nativella\Notification` shows desktop notifications.

```php
$notification->send('Export finished', 'notes.zip is in your Downloads folder.');
```

| Method | |
|---|---|
| `send(string $title, ?string $body = null, ?string $reference = null): void` | Shows a notification, with an optional body. |

`send()` returns as soon as the system has the notification: it does not wait for the user.

## Clicks

A notification sent with a reference tells PHP when the user clicks it, with a `Nativella\Event\NotificationClicked` event that carries the reference:

```php
$notification->send('New message', 'From Alice', reference: 'message-42');
```

```php
use Nativella\Event\NotificationClicked;

#[AsEventListener]
public function onClick(NotificationClicked $event): void
{
    // $event->reference === 'message-42'
    $this->window->open('main');
}
```

Linux only, for now: macOS and Windows show the notification, but do not report the click.

On Linux, notifications go through the notification service of the desktop (`org.freedesktop.Notifications`, on D-Bus), which every mainstream desktop provides.

## A notification from a page

A notification needs no page change: a `fetch()` from the page is enough.

```js
await fetch('/reminders/42/notify', { method: 'POST' });
```

```php
// POST /reminders/{id}/notify
$notification->send('Reminder', $reminder->title);

http_response_code(204);
```

## A notification from the background

PHP only runs when something asks it to: a request from a page, or a [native event](lifecycle.md#native-events). A reminder "in ten minutes" is a [timer](timers.md): `$timer->after('reminder-42', 600)`, then the notification from the listener of `TimerFired`.
