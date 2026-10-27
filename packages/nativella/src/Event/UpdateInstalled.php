<?php

namespace Nativella\Event;

/**
 * Updater::install() replaced the executable: the next start runs `$version`.
 */
final readonly class UpdateInstalled
{
    public function __construct(
        public string $version,
    ) {
    }
}
