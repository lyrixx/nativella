<?php

namespace Nativella\Event;

/**
 * Files were dragged from the system and dropped on a window.
 */
final readonly class WindowFilesDropped
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
