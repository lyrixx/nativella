<?php

namespace Nativella\Event;

use Nativella\Tray\MouseButton;

final readonly class TrayClicked
{
    public function __construct(
        public MouseButton $button,
    ) {
    }
}
