<?php

namespace Nativella;

use Nativella\Geolocation\Accuracy;
use Nativella\Geolocation\Position;

/**
 * Where the phone is.
 *
 * The application asks for it with "geolocation" in the permissions of
 * extra.nativella, in its composer.json; the user is asked on the first call.
 */
final readonly class Geolocation
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Null when the user does not allow it. Blocks until the phone knows: a
     * few seconds at worst.
     *
     * @param int $maximumAge in seconds: how old a position the phone already knows may be, 0 for a new one
     *
     * @throws Exception\NativeCallException when the location is turned off, or the phone finds no position within 20 seconds
     */
    public function currentPosition(Accuracy $accuracy = Accuracy::Balanced, int $maximumAge = 0): ?Position
    {
        $position = $this->client->call('geolocation.currentPosition', [
            'accuracy' => $accuracy->value,
            'maximumAge' => $maximumAge,
        ])['position'];

        return null === $position ? null : Position::fromArray($position);
    }
}
