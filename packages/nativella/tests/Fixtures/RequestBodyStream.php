<?php

namespace Nativella\Tests\Fixtures;

/**
 * Stands in for php://input, which is empty on the command line: registered
 * as the "php" stream wrapper, it gives $body to whoever reads a php:// stream.
 */
final class RequestBodyStream
{
    public static string $body = '';

    /** @var resource|null */
    public $context;

    private int $position = 0;

    public function stream_open(): bool
    {
        return true;
    }

    public function stream_read(int $count): string
    {
        $chunk = substr(self::$body, $this->position, $count);
        $this->position += \strlen($chunk);

        return $chunk;
    }

    public function stream_eof(): bool
    {
        return $this->position >= \strlen(self::$body);
    }

    /**
     * @return array{}
     */
    public function stream_stat(): array
    {
        return [];
    }
}
