<?php

namespace Nativella\Event;

/**
 * A native event as the runtime sent it. Symfony and Laravel dispatch every
 * event as one, after its own class when this version of the package has one:
 * listen to it for an event the package does not know yet, or for all of them.
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
