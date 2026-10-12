<?php

namespace Nativella\Event;

final readonly class WindowBlurred
{
    public function __construct(
        public string $window,
    ) {
    }
}
