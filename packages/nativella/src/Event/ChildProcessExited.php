<?php

namespace Nativella\Event;

/**
 * A process started with ChildProcess::start() ended, after its last
 * ChildProcessOutput.
 */
final readonly class ChildProcessExited
{
    public function __construct(
        public string $alias,
        /** Tells a process from the one started again under its alias. */
        public int $pid,
        /** Null when a signal ended it. */
        public ?int $code,
    ) {
    }
}
