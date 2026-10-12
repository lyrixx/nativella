<?php

namespace Nativella\Website;

/**
 * What every page knows about the project.
 */
final readonly class SiteInfo
{
    public function __construct(
        public string $repository,
        /** Where the site is published, with a trailing slash. */
        public string $url,
        public string $version,
    ) {
    }
}
