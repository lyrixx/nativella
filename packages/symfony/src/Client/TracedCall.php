<?php

namespace Nativella\Symfony\Client;

/**
 * A native call of the request, as TraceableClient recorded it.
 */
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
        /** The message of the exception, when it failed. */
        public ?string $error,
        public float $durationInMs,
    ) {
    }
}
