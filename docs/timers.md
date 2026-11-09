# Timers

PHP only runs to answer a request or a [native event](lifecycle.md#native-events), so nothing happens while the user does nothing. `Nativella\Timer` wakes it up for periodic work (a sync, a cleanup, a reminder) with a `Nativella\Event\TimerFired` event.

```php
use Nativella\Event\AppBooted;
use Nativella\Event\TimerFired;

#[AsEventListener]
public function onBoot(AppBooted $event): void
{
    $this->timer->every('sync', 5 * 60);
}

#[AsEventListener]
public function onTimer(TimerFired $event): void
{
    if ('sync' === $event->name) {
        $this->synchronizer->synchronize();
    }
}
```

| Method | |
|---|---|
| `every(string $name, float $seconds): void` | Fires every `$seconds`, from now on. |
| `after(string $name, float $seconds): void` | Fires once, in `$seconds`. |
| `cancel(string $name): void` | Stops a timer. Nothing happens if there is none of that name. |

A timer fires every second at most, and once a year at least. Starting a timer replaces the one with the same name.

Timers live as long as the application, so start them when it [boots](lifecycle.md#booting). A timer fires on time, but its event waits in line with the requests, because PHP serves one at a time. Keep the work short, or split it, or the pages wait for it. A tick that comes before PHP has handled the previous one, behind an open [dialog](dialogs.md) for instance, is skipped, so ticks do not pile up.

## Instead of a scheduler and a queue

There is no cron and no queue worker in a desktop application. A timer covers what a scheduler would. Run your scheduler's commands from its listener (`schedule:run` for Laravel, the messages of a Symfony Scheduler), or process a few queue messages at each tick.
