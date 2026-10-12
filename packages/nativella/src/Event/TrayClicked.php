<?php

namespace Nativella\Event;

final readonly class TrayClicked
{
    public function __construct(
        /** "left", "right" or "middle". */
        public string $button,
    ) {
    }
}
