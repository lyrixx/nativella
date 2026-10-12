<?php

namespace Nativella\Event;

final readonly class GlobalShortcutPressed
{
    public function __construct(
        /** As it was registered. */
        public string $accelerator,
    ) {
    }
}
