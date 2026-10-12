<?php

namespace Nativella\Screen;

final readonly class Display
{
    public function __construct(
        public ?string $name,
        public bool $primary,
        public Rectangle $bounds,
        /** Without the taskbar, the dock or the menu bar. */
        public Rectangle $workArea,
        public float $scaleFactor,
    ) {
    }

    /**
     * @param array{name: ?string, primary: bool, x: int|float, y: int|float, width: int|float, height: int|float, scaleFactor: int|float, workArea: array{x: int|float, y: int|float, width: int|float, height: int|float}} $data
     */
    public static function fromArray(array $data): self
    {
        return new self(
            $data['name'],
            $data['primary'],
            Rectangle::fromArray($data),
            Rectangle::fromArray($data['workArea']),
            (float) $data['scaleFactor'],
        );
    }
}
