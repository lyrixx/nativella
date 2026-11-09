<?php

namespace Nativella\Event;

/**
 * A window's download was saved where the user chose. `$path` is absolute.
 */
final readonly class WindowDownloaded
{
    public function __construct(
        public string $window,
        public string $path,
    ) {
    }
}
