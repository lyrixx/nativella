<?php

namespace Nativella;

/**
 * Talks to the JavaScript of the pages, which listen with:
 *
 *     window.addEventListener('nativella:note.saved', (event) => console.log(event.detail));
 *
 * Native events (menu clicks, shortcuts…) reach the pages the same way.
 */
final readonly class Page
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * @param array<string, mixed> $detail
     * @param string|null          $window every window when null
     */
    public function dispatch(string $event, array $detail = [], ?string $window = null): void
    {
        $this->client->call('page.dispatch', ['event' => $event, 'detail' => (object) $detail, 'window' => $window]);
    }
}
