<?php

namespace Nativella\Window;

/**
 * How Window::open() opens a window. Null leaves the default of the system.
 *
 * Keep in sync with WindowOptions in crates/nativella/src/native/window.rs.
 */
final readonly class WindowOptions
{
    public function __construct(
        /** A path of the application ("/settings"), or an absolute URL; "/" when null. */
        public ?string $url = null,
        /** The name of the application when null. */
        public ?string $title = null,
        public ?float $width = null,
        public ?float $height = null,
        public ?float $minWidth = null,
        public ?float $minHeight = null,
        public ?float $maxWidth = null,
        public ?float $maxHeight = null,
        public ?float $x = null,
        public ?float $y = null,
        public ?bool $center = null,
        public ?bool $resizable = null,
        public ?bool $maximizable = null,
        public ?bool $minimizable = null,
        public ?bool $closable = null,
        public ?bool $maximized = null,
        public ?bool $fullscreen = null,
        /** False for a frameless window. */
        public ?bool $decorations = null,
        public ?bool $alwaysOnTop = null,
        public ?bool $focused = null,
        public ?bool $visible = null,
        public ?bool $skipTaskbar = null,
    ) {
    }

    /**
     * @return array<string, scalar> the options as the runtime reads them
     */
    public function toArray(): array
    {
        return array_filter(get_object_vars($this), static fn (mixed $value): bool => null !== $value);
    }
}
