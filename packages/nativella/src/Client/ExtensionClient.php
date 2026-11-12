<?php

namespace Nativella\Client;

use Nativella\Client;
use Nativella\Exception\NativeCallException;
use Nativella\Exception\RuntimeUnavailableException;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\Runtime;

/**
 * Calls the runtime through the nativella_call() function it registers in PHP.
 */
final readonly class ExtensionClient implements Client
{
    public function call(string $method, array $payload = []): array
    {
        if (!Runtime::isAvailable()) {
            throw new RuntimeUnavailableException(\sprintf('Cannot call "%s": the application does not run in Nativella.', $method));
        }

        try {
            $json = json_encode((object) $payload, \JSON_THROW_ON_ERROR | \JSON_PRESERVE_ZERO_FRACTION | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE);
        } catch (\JsonException $e) {
            // A string that is not UTF-8: text read from a file in another encoding.
            throw new NativeCallException($method, $e);
        }

        try {
            $result = nativella_call($method, $json);
        } catch (\RuntimeException $e) {
            throw new NativeCallException($method, $e);
        }

        $decoded = json_decode($result, true);
        if (!\is_array($decoded)) {
            throw new UnexpectedResponseException(\sprintf('The runtime answered "%s" with something that is not a JSON object.', $method));
        }

        return $decoded;
    }

    public function isNative(): bool
    {
        return Runtime::isAvailable();
    }
}
