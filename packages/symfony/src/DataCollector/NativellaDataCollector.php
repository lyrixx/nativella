<?php

namespace Nativella\Symfony\DataCollector;

use Nativella\Runtime;
use Nativella\Symfony\Client\TraceableClient;
use Nativella\Symfony\Client\TracedCall;
use Symfony\Bundle\FrameworkBundle\DataCollector\AbstractDataCollector;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;

final class NativellaDataCollector extends AbstractDataCollector
{
    public function __construct(
        private readonly TraceableClient $client,
    ) {
    }

    public function collect(Request $request, Response $response, ?\Throwable $exception = null): void
    {
        $calls = $this->client->calls();

        $this->data = [
            'calls' => array_map(fn (TracedCall $call): array => [
                'method' => $call->method,
                'payload' => $this->cloneVar(self::mask($call->method, $call->payload)),
                'result' => null === $call->result ? null : $this->cloneVar(self::mask($call->method, $call->result)),
                'error' => $call->error,
                'durationInMs' => $call->durationInMs,
            ], $calls),
            'call_count' => \count($calls),
            'error_count' => \count(array_filter($calls, static fn (TracedCall $call): bool => null !== $call->error)),
            'duration' => array_sum(array_column($calls, 'durationInMs')),
            'native' => Runtime::isNative(),
            'version' => $request->server->get('NATIVELLA_VERSION'),
            'window' => $request->server->get('NATIVELLA_WINDOW'),
            'data_dir' => $request->server->get('NATIVELLA_DATA_DIR'),
        ];
    }

    public static function getTemplate(): string
    {
        return '@Nativella/Collector/nativella.html.twig';
    }

    /**
     * @return list<array{method: string, payload: mixed, result: mixed, error: string|null, durationInMs: float}>
     */
    public function getCalls(): array
    {
        return $this->data['calls'];
    }

    public function getCallCount(): int
    {
        return $this->data['call_count'];
    }

    public function getErrorCount(): int
    {
        return $this->data['error_count'];
    }

    public function getDuration(): float
    {
        return $this->data['duration'];
    }

    public function isNative(): bool
    {
        return $this->data['native'];
    }

    public function getVersion(): ?string
    {
        return $this->data['version'];
    }

    public function getWindow(): ?string
    {
        return $this->data['window'];
    }

    public function getDataDir(): ?string
    {
        return $this->data['data_dir'];
    }

    /**
     * Profiles are kept on disk, so keychain secrets stay out of them.
     *
     * @param array<string, mixed> $data
     *
     * @return array<string, mixed>
     */
    private static function mask(string $method, array $data): array
    {
        if (str_starts_with($method, 'secrets.') && isset($data['value'])) {
            $data['value'] = '******';
        }

        return $data;
    }
}
