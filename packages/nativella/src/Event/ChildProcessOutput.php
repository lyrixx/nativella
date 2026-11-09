<?php

namespace Nativella\Event;

use Nativella\ChildProcess\Stream;

/**
 * $output comes as the process wrote it, often whole lines with their line
 * endings, sometimes a prompt or a part of a line.
 */
final readonly class ChildProcessOutput
{
    public function __construct(
        public string $alias,
        /** Tells a process from the one started again under its alias. */
        public int $pid,
        public Stream $stream,
        public string $output,
    ) {
    }
}
