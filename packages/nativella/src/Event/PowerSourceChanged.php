<?php

namespace Nativella\Event;

/**
 * The computer went on battery, or back on mains.
 */
final readonly class PowerSourceChanged
{
    public function __construct(
        public bool $onBattery,
    ) {
    }
}
