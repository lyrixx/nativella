<?php

namespace Nativella\Event;

/**
 * Updater::install() replaced the executable, so the next start runs `$version`.
 */
final readonly class UpdaterInstalled
{
    public function __construct(
        public string $version,
    ) {
    }
}
