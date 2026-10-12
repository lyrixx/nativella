<?php

namespace Nativella\Window;

/**
 * A window as it is now. Sizes and positions are in logical pixels.
 */
final readonly class WindowInfo
{
    public function __construct(
        public string $label,
        public ?string $title,
        public ?string $url,
        public ?float $width,
        public ?float $height,
        public ?float $x,
        public ?float $y,
        public bool $focused,
        public bool $maximized,
        public bool $minimized,
        public bool $fullscreen,
        public bool $visible,
    ) {
    }

    /**
     * @param array{label: string, title: ?string, url: ?string, width: int|float|null, height: int|float|null, x: int|float|null, y: int|float|null, focused: bool, maximized: bool, minimized: bool, fullscreen: bool, visible: bool} $data
     */
    public static function fromArray(array $data): self
    {
        $float = static fn (int|float|null $value): ?float => null === $value ? null : (float) $value;

        return new self(
            $data['label'],
            $data['title'],
            $data['url'],
            $float($data['width']),
            $float($data['height']),
            $float($data['x']),
            $float($data['y']),
            $data['focused'],
            $data['maximized'],
            $data['minimized'],
            $data['fullscreen'],
            $data['visible'],
        );
    }
}
