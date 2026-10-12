# Power

`Nativella\Power` tells whether the computer runs on battery, and how much is left: to put off a heavy sync until it is plugged in, or to warn before a long export.

```php
if ($power->isOnBattery() && $power->battery()?->level < 0.2) {
    $notification->send('Low battery', 'The backup will run once the computer is plugged in.');
}
```

| Method | |
|---|---|
| `isOnBattery(): bool` | Whether the computer runs on its battery: false on mains, and without a battery. |
| `battery(): ?Battery` | The battery, null without one (a desktop computer). Several batteries count as one. |

A `Battery` has a `level`, from 0 to 1, a `state` (`BatteryState::Charging`, `Discharging`, `Full`, `Empty`, `NotCharging` when a charge limit holds it on mains, or `Unknown`), and `timeToEmpty` and `timeToFull` in seconds, when the system has an estimate.

## Events

| Event | When |
|---|---|
| `PowerSourceChanged` | The computer went on battery, or back on mains: `$event->onBattery`. |
| `PowerResumed` | The computer woke from sleep: data fetched before may be stale, a connection may be dead. |

Nativella looks every 5 seconds, and reports sleeps of 15 seconds or more. On Linux, it reads how long the computer slept from the system. Elsewhere, it compares the wall clock to the monotonic one: setting the clock forward looks like a sleep, and Windows, whose monotonic clock runs during sleep, never reports one.

Desktop only: on a phone, `Power` throws a `NativeCallException`.
