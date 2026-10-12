<?php

namespace Nativella\Event;

final readonly class WindowDragLeft
{
    public function __construct(
        public string $window,
    ) {
    }
}
