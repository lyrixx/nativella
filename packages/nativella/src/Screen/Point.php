<?php

namespace Nativella\Screen;

/**
 * In physical pixels, from the top left corner of the primary display.
 */
final readonly class Point
{
    public function __construct(
        public float $x,
        public float $y,
    ) {
    }
}
