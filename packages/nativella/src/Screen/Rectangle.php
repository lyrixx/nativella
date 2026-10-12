<?php

namespace Nativella\Screen;

/**
 * In logical pixels, as CSS and window sizes are.
 */
final readonly class Rectangle
{
    public function __construct(
        public float $x,
        public float $y,
        public float $width,
        public float $height,
    ) {
    }

    /**
     * @param array{x: int|float, y: int|float, width: int|float, height: int|float} $data
     */
    public static function fromArray(array $data): self
    {
        return new self((float) $data['x'], (float) $data['y'], (float) $data['width'], (float) $data['height']);
    }
}
