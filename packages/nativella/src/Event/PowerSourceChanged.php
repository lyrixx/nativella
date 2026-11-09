<?php

namespace Nativella\Event;

final readonly class PowerSourceChanged
{
    public function __construct(
        public bool $onBattery,
    ) {
    }
}
