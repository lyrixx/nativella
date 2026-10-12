<?php

namespace Nativella\Event;

use Nativella\ChildProcess\Stream;

/**
 * A process started with ChildProcess::start() wrote something, as it came:
 * often whole lines with their line endings, sometimes a prompt or a part of
 * a line.
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
