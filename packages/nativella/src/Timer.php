<?php

namespace Nativella;

/**
 * Wakes PHP up: PHP only runs to answer requests and native events, so
 * periodic work (a sync, a cleanup, a reminder) needs a timer. Each one
 * dispatches Event\TimerFired with its name.
 *
 * Timers live as long as the application: register them when it boots.
 */
final readonly class Timer
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Fires every $seconds, from now on. A timer of the same name is replaced.
     *
     * @param float $seconds from 1 second to 1 year
     */
    public function every(string $name, float $seconds): void
    {
        $this->client->call('timer.every', ['name' => $name, 'seconds' => $seconds]);
    }

    /**
     * Fires once, in $seconds. A timer of the same name is replaced.
     *
     * @param float $seconds from 1 second to 1 year
     */
    public function after(string $name, float $seconds): void
    {
        $this->client->call('timer.after', ['name' => $name, 'seconds' => $seconds]);
    }

    public function cancel(string $name): void
    {
        $this->client->call('timer.cancel', ['name' => $name]);
    }
}
