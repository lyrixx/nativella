<?php

namespace Nativella;

/**
 * Calls a native method of the Nativella runtime.
 *
 * Methods are named "<domain>.<method>" ("window.open"), payloads and results
 * are JSON objects with camelCase keys. Keep in sync with the native methods
 * in crates/nativella/src/native/.
 */
interface Client
{
    /**
     * @param array<string, mixed> $payload
     *
     * @return array<string, mixed>
     *
     * @throws Exception\NativeCallException         when the native side fails
     * @throws Exception\RuntimeUnavailableException when PHP does not run in Nativella
     */
    public function call(string $method, array $payload = []): array;
}
