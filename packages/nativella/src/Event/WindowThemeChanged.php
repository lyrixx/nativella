<?php

namespace Nativella\Event;

use Nativella\System\Theme;

final readonly class WindowThemeChanged
{
    public function __construct(
        public string $window,
        public Theme $theme,
    ) {
    }
}
