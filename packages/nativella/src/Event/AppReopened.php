<?php

namespace Nativella\Event;

/**
 * The user launched the application again while it was running. The new
 * instance exits, and the running one has already focused a window.
 */
final readonly class AppReopened
{
    /**
     * @param list<string> $args the command line of the new instance
     */
    public function __construct(
        public array $args,
        public string $cwd,
    ) {
    }
}
