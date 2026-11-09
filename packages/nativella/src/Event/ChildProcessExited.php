<?php

namespace Nativella\Event;

/**
 * Comes after the process's last ChildProcessOutput.
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
