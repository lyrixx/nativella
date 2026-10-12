<?php

namespace Nativella\Event;

/**
 * The system opened URLs of a deep link scheme of the application
 * ("notes://42", see "deepLinks" in extra.nativella): at launch, or later
 * when the application already runs.
 */
final readonly class AppUrlsOpened
{
    /**
     * @param non-empty-list<string> $urls the system never opens zero URLs
     */
    public function __construct(
        public array $urls,
    ) {
    }
}
