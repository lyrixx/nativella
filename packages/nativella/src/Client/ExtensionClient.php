<?php

namespace Nativella\Client;

use Nativella\Client;
use Nativella\Exception\NativeCallException;
use Nativella\Exception\RuntimeUnavailableException;
use Nativella\Runtime;

/**
 * Calls the runtime through the nativella_call() function it registers in PHP.
 */
final readonly class ExtensionClient implements Client
{
    public function call(string $method, array $payload = []): array
    {
        if (!Runtime::isNative()) {
            throw new RuntimeUnavailableException(\sprintf('Cannot call "%s": the application does not run in Nativella.', $method));
        }

        $json = json_encode((object) $payload, \JSON_THROW_ON_ERROR | \JSON_PRESERVE_ZERO_FRACTION | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE);

        try {
            $result = nativella_call($method, $json);
        } catch (\RuntimeException $e) {
            throw new NativeCallException($method, $e);
        }

        return json_decode($result, true, flags: \JSON_THROW_ON_ERROR);
    }
}
