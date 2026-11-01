<?php

namespace Nativella\Client;

/**
 * A native call, as FakeClient recorded it.
 */
final readonly class Call
{
    /**
     * @param array<string, mixed> $payload
     */
    public function __construct(
        /** "<domain>.<method>", as "window.open". */
        public string $method,
        public array $payload,
    ) {
    }
}
