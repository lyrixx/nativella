<?php

namespace Nativella\Event;

/**
 * A download of a window was saved where the user chose: `$path`, absolute.
 */
final readonly class WindowDownloaded
{
    public function __construct(
        public string $window,
        public string $path,
    ) {
    }
}
