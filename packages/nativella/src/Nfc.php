<?php

namespace Nativella;

use Nativella\Nfc\Tag;

/**
 * The application asks for it with "nfc" in extra.nativella.permissions, in
 * its composer.json.
 */
final readonly class Nfc
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Whether the phone has NFC, turned on. False on a computer, and on a
     * phone whose application does not ask for "nfc".
     */
    public function isAvailable(): bool
    {
        return $this->client->call('nfc.isAvailable')['available'];
    }

    /**
     * Waits for a tag to come near the phone, and reads it.
     *
     * @throws Exception\NativeCallException when NFC is not available, or no tag comes within 60 seconds
     */
    public function read(): Tag
    {
        return Tag::fromArray($this->client->call('nfc.read')['tag']);
    }
}
