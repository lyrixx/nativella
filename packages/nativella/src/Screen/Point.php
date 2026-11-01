<?php

namespace Nativella\Screen;

/**
 * In physical pixels, from the top left corner of the desktop the displays
 * share: the one of the primary display on Windows and macOS, of the leftmost
 * display on X11.
 */
final readonly class Point
{
    public function __construct(
        public float $x,
        public float $y,
    ) {
    }
}
