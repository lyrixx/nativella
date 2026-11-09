<?php

namespace Nativella\Symfony\Client;

final readonly class TracedCall
{
    /**
     * @param array<string, mixed>      $payload
     * @param array<string, mixed>|null $result  null when it failed
     */
    public function __construct(
        public string $method,
        public array $payload,
        public ?array $result,
        /** The exception's message, when it failed. */
        public ?string $error,
        public float $durationInMs,
    ) {
    }
}
