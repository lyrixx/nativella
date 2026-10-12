<?php

namespace Nativella;

use Nativella\Haptics\Feedback;
use Nativella\Haptics\Impact;

/**
 * Vibrations and haptic feedback, on phones.
 *
 * The application asks for them with "haptics" in the permissions of
 * extra.nativella, in its composer.json.
 */
final readonly class Haptics
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function vibrate(int $milliseconds = 300): void
    {
        $this->client->call('haptics.vibrate', ['milliseconds' => $milliseconds]);
    }

    /**
     * A tap, as when an element snaps into place.
     */
    public function impact(Impact $impact = Impact::Medium): void
    {
        $this->client->call('haptics.impact', ['impact' => $impact->value]);
    }

    /**
     * The pattern of a success, a warning or an error.
     */
    public function feedback(Feedback $feedback): void
    {
        $this->client->call('haptics.feedback', ['feedback' => $feedback->value]);
    }

    /**
     * A tick, as when a selection changes.
     */
    public function selection(): void
    {
        $this->client->call('haptics.selection');
    }
}
