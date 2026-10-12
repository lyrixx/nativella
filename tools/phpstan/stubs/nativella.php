<?php

// What the runtime registers in PHP, for PHPStan: the projects run on it.
// Keep in sync with crates/runtime/src/bridge.rs.

/**
 * Calls a native method with a JSON payload, and returns its JSON result.
 *
 * @throws RuntimeException when the native side fails
 */
function nativella_call(string $method, string $payload): string
{
}
