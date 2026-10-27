<?php

namespace Nativella\Event;

/**
 * Updater::install() did not install `$version`: the download failed, or its
 * signature did not match.
 */
final readonly class UpdateFailed
{
    public function __construct(
        public string $version,
        public string $message,
    ) {
    }
}
