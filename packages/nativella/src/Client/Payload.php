<?php

namespace Nativella\Client;

use Nativella\Exception\NativeCallException;

/**
 * @internal
 */
final readonly class Payload
{
    private function __construct()
    {
    }

    /**
     * @param array<string, mixed> $payload
     *
     * @throws NativeCallException when JSON cannot hold the payload
     */
    public static function encode(string $method, array $payload): string
    {
        try {
            return json_encode((object) $payload, \JSON_THROW_ON_ERROR | \JSON_PRESERVE_ZERO_FRACTION | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE);
        } catch (\JsonException $e) {
            // A string that is not UTF-8 (text read from a file in another
            // encoding), INF or NAN.
            throw new NativeCallException($method, $e);
        }
    }
}
