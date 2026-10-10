<?php

namespace App\Native;

/**
 * Calls the host application through the native_call() function that the
 * native-php runtime registers in PHP.
 *
 * Keep the method names in sync with crates/desktop/src/native.rs.
 */
final readonly class Native
{
    public function isAvailable(): bool
    {
        return \function_exists('native_call');
    }

    /**
     * @param array<string, mixed> $payload
     *
     * @return array<string, mixed>
     */
    public function call(string $method, array $payload = []): array
    {
        if (!$this->isAvailable()) {
            throw new \LogicException(\sprintf('Cannot call "%s": the application does not run in the native-php runtime.', $method));
        }

        $result = native_call($method, json_encode((object) $payload, \JSON_THROW_ON_ERROR));

        return json_decode($result, true, flags: \JSON_THROW_ON_ERROR);
    }
}
