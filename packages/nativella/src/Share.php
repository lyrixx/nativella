<?php

namespace Nativella;

/**
 * The share sheet of the phone: the user sends a text or a file to another
 * application, or to a contact. No permission needed.
 *
 * The sheet opens and the call returns: nothing tells whether the user shared
 * or closed it.
 */
final readonly class Share
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * A text, a link: the receiving application makes what it can of it.
     */
    public function text(string $text): void
    {
        $this->client->call('share.text', ['text' => $text]);
    }

    /**
     * @param string      $path     absolute, readable by PHP: a file of the data directory
     * @param string|null $mimeType "text/csv", "image/png"… which tells the phone what can open it; any when null
     */
    public function file(string $path, ?string $mimeType = null, ?string $title = null): void
    {
        $this->client->call('share.file', array_filter(
            ['path' => $path, 'mimeType' => $mimeType, 'title' => $title],
            static fn (?string $value): bool => null !== $value,
        ));
    }
}
