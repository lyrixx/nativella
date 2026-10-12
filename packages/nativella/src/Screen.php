<?php

namespace Nativella;

use Nativella\Screen\Display;
use Nativella\Screen\Point;

/**
 * The displays and the cursor.
 */
final readonly class Screen
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * @return list<Display>
     */
    public function all(): array
    {
        // @phpstan-ignore return.type (a JSON array, so a list: crates/nativella/src/native/screen.rs)
        return array_map(Display::fromArray(...), $this->client->call('screen.all')['screens']);
    }

    /**
     * Null when the system has no primary display: Wayland sessions, among
     * others, where Display::$primary is always false too.
     */
    public function primary(): ?Display
    {
        $screen = $this->client->call('screen.primary')['screen'];

        return null === $screen ? null : Display::fromArray($screen);
    }

    public function cursorPosition(): Point
    {
        $position = $this->client->call('screen.cursorPosition');

        return new Point((float) $position['x'], (float) $position['y']);
    }
}
