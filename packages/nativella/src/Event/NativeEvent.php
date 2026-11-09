<?php

namespace Nativella\Event;

/**
 * An event this version of the package does not know, sent by a newer runtime.
 */
final readonly class NativeEvent
{
    /**
     * @param array<string, mixed> $payload
     */
    public function __construct(
        public string $name,
        public array $payload,
    ) {
    }
}
