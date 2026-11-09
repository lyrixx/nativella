<?php

namespace Nativella\Symfony\Client;

use Nativella\Client;
use Symfony\Contracts\Service\ResetInterface;

final class TraceableClient implements Client, ResetInterface
{
    /** @var list<TracedCall> */
    private array $calls = [];

    public function __construct(
        private readonly Client $client,
    ) {
    }

    public function call(string $method, array $payload = []): array
    {
        $start = hrtime(true);

        try {
            $result = $this->client->call($method, $payload);
            $this->record($method, $payload, $result, null, $start);

            return $result;
        } catch (\Throwable $e) {
            $this->record($method, $payload, null, $e->getMessage(), $start);

            throw $e;
        }
    }

    public function isNative(): bool
    {
        return $this->client->isNative();
    }

    /**
     * @return list<TracedCall>
     */
    public function calls(): array
    {
        return $this->calls;
    }

    public function reset(): void
    {
        $this->calls = [];
    }

    /**
     * @param array<string, mixed>      $payload
     * @param array<string, mixed>|null $result
     */
    private function record(string $method, array $payload, ?array $result, ?string $error, int $start): void
    {
        $this->calls[] = new TracedCall($method, $payload, $result, $error, (hrtime(true) - $start) / 1e6);
    }
}
