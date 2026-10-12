<?php

namespace Nativella;

/**
 * Tokens and passwords, kept in the keychain of the system (Secret Service
 * on Linux, the Keychain on macOS, the Credential Manager on Windows), which
 * encrypts them, rather than in the database or the settings. Desktop only.
 */
final readonly class Secrets
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Null when there is no such secret.
     */
    public function get(string $key): ?string
    {
        return $this->client->call('secrets.get', ['key' => $key])['value'];
    }

    public function set(string $key, string $value): void
    {
        $this->client->call('secrets.set', ['key' => $key, 'value' => $value]);
    }

    /**
     * Nothing happens if there is no such secret.
     */
    public function delete(string $key): void
    {
        $this->client->call('secrets.delete', ['key' => $key]);
    }
}
