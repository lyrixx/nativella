<?php

namespace App\Native;

use Symfony\Component\DependencyInjection\Attribute\Autowire;
use Symfony\Component\Filesystem\Filesystem;

/**
 * Keeps the native events PHP received, to show them on the demo page.
 */
final readonly class NativeEventLog
{
    public function __construct(
        private Filesystem $filesystem,
        #[Autowire('%kernel.logs_dir%/native-events.jsonl')]
        private string $path,
    ) {
    }

    public function record(NativeEvent $event): void
    {
        $line = json_encode([
            'name' => $event->name,
            'payload' => $event->payload,
            'receivedAt' => $event->receivedAt->format(\DATE_ATOM),
        ], \JSON_THROW_ON_ERROR);

        $this->filesystem->appendToFile($this->path, $line."\n", lock: true);
    }

    /**
     * @return list<NativeEvent> the most recent first
     */
    public function latest(int $limit): array
    {
        if (!is_file($this->path)) {
            return [];
        }

        $lines = file($this->path, \FILE_IGNORE_NEW_LINES | \FILE_SKIP_EMPTY_LINES);
        if (false === $lines) {
            throw new \RuntimeException(\sprintf('Could not read the native event log "%s".', $this->path));
        }

        $events = [];
        foreach (\array_slice(array_reverse($lines), 0, $limit) as $line) {
            $data = json_decode($line, true, flags: \JSON_THROW_ON_ERROR);
            $events[] = new NativeEvent($data['name'], $data['payload'], new \DateTimeImmutable($data['receivedAt']));
        }

        return $events;
    }
}
