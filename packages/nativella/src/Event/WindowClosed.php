<?php

namespace Nativella\Event;

final readonly class WindowClosed
{
    public function __construct(
        public string $window,
    ) {
    }
}
