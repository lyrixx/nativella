<?php

namespace Nativella\Screen;

/**
 * In physical pixels, from the top left corner of the desktop the displays
 * share. That is the primary display's corner on Windows and macOS, and the
 * leftmost display's on X11.
 */
final readonly class Point
{
    public function __construct(
        public float $x,
        public float $y,
    ) {
    }
}
