<?php

namespace Nativella;

use Nativella\Power\Battery;

/**
 * The battery and the power source of the computer, and keeping it awake.
 * Events tell when they change: Event\PowerSourceChanged, and
 * Event\PowerResumed when the computer wakes from sleep. Desktop only.
 */
final readonly class Power
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function isOnBattery(): bool
    {
        return $this->client->call('power.status')['onBattery'];
    }

    /**
     * Null without a battery: a desktop computer. Several batteries count as
     * one.
     */
    public function battery(): ?Battery
    {
        $battery = $this->client->call('power.status')['battery'];

        return null === $battery ? null : Battery::fromArray($battery);
    }

    /**
     * Keeps the screen on and the computer awake until allowSleep(), or until
     * the application quits: during a presentation, a long export.
     *
     * @param string $reason what the system shows the user, as "Exporting the notes"
     */
    public function keepAwake(string $reason): void
    {
        $this->client->call('power.keepAwake', ['reason' => $reason]);
    }

    public function allowSleep(): void
    {
        $this->client->call('power.allowSleep');
    }
}
