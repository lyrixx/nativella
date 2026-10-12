<?php

namespace Nativella\Event;

final readonly class WindowFocused
{
    public function __construct(
        public string $window,
    ) {
    }
}
