<?php

namespace Nativella;

/**
 * A small key-value store for preferences, kept by the runtime in the data
 * directory. Values are anything JSON can hold. Application data belongs in
 * the application's database.
 */
final readonly class Settings
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function get(string $key, mixed $default = null): mixed
    {
        // A missing key has no "value" at all, which a null value is not.
        $result = $this->client->call('settings.get', ['key' => $key]);

        return \array_key_exists('value', $result) ? $result['value'] : $default;
    }

    public function has(string $key): bool
    {
        return $this->client->call('settings.has', ['key' => $key])['has'];
    }

    public function set(string $key, mixed $value): void
    {
        $this->client->call('settings.set', ['key' => $key, 'value' => $value]);
    }

    public function forget(string $key): void
    {
        $this->client->call('settings.forget', ['key' => $key]);
    }

    /**
     * @return array<string, mixed>
     */
    public function all(): array
    {
        return $this->client->call('settings.all')['settings'];
    }

    public function clear(): void
    {
        $this->client->call('settings.clear');
    }
}
