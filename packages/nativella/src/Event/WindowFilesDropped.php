<?php

namespace Nativella\Event;

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
