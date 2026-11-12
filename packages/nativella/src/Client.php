<?php

namespace Nativella;

/**
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
     * @throws Exception\NativeCallException         when the native side fails, or the payload holds a string that is not UTF-8
     * @throws Exception\RuntimeUnavailableException when PHP does not run in Nativella
     * @throws Exception\UnexpectedResponseException when the native side answers something the package cannot read
     * @throws Exception\VersionMismatchException    when the nativella command and the package are not of the same release line
     */
    public function call(string $method, array $payload = []): array;

    /**
     * Native calls work when PHP runs in Nativella, or when the client is a
     * fake, under which the application behaves as in Nativella.
     */
    public function isNative(): bool;
}
