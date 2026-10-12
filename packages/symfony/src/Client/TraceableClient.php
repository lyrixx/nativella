<?php

namespace Nativella\Symfony\Client;

use Nativella\Client;
use Symfony\Contracts\Service\ResetInterface;

/**
 * Records the native calls of the request for the profiler.
 */
final class TraceableClient implements Client, ResetInterface
{
    /** @var list<array{method: string, payload: array<string, mixed>, result: array<string, mixed>|null, error: string|null, duration: float}> */
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

    /**
     * @return list<array{method: string, payload: array<string, mixed>, result: array<string, mixed>|null, error: string|null, duration: float}>
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
        $this->calls[] = [
            'method' => $method,
            'payload' => $payload,
            'result' => $result,
            'error' => $error,
            'duration' => (hrtime(true) - $start) / 1e6,
        ];
    }
}
