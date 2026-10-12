<?php

namespace Nativella\Event;

/**
 * Files dragged from the system entered a window: the page may highlight
 * where to drop them, until WindowDragLeft or WindowFilesDropped.
 */
final readonly class WindowDragEntered
{
    /**
     * @param list<string> $paths absolute
     */
    public function __construct(
        public string $window,
        public array $paths,
    ) {
    }
}
