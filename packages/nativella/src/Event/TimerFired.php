<?php

namespace Nativella\Event;

final readonly class TimerFired
{
    public function __construct(
        /** As given to Timer::every() or Timer::after(). */
        public string $name,
    ) {
    }
}
