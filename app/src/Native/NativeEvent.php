<?php

namespace App\Native;

/**
 * Something that happened on the native side (a menu click, a window
 * focus...), dispatched as "native.<name>".
 */
final readonly class NativeEvent
{
    /**
     * @param array<string, mixed> $payload
     */
    public function __construct(
        public string $name,
        public array $payload,
        public \DateTimeImmutable $receivedAt = new \DateTimeImmutable(),
    ) {
    }
}
