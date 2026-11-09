<?php

namespace Nativella;

use Nativella\Screen\Display;
use Nativella\Screen\Point;

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
        return array_map(Display::fromArray(...), $this->client->call('screen.all')['displays']);
    }

    /**
     * Null when the system has no primary display, as on Wayland sessions
     * among others, where Display::$primary is always false too.
     */
    public function primary(): ?Display
    {
        $display = $this->client->call('screen.primary')['display'];

        return null === $display ? null : Display::fromArray($display);
    }

    public function cursorPosition(): Point
    {
        $position = $this->client->call('screen.cursorPosition');

        return new Point((float) $position['x'], (float) $position['y']);
    }
}
