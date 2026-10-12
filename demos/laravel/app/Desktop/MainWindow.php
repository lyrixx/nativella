<?php

namespace App\Desktop;

use Nativella\Window;

/**
 * The window Nativella opens at boot.
 */
final readonly class MainWindow
{
    /**
     * Keep in sync with MAIN_WINDOW in crates/nativella/src/lib.rs.
     */
    private const string LABEL = 'main';

    public function __construct(
        private Window $window,
    ) {
    }

    /**
     * Brings it to the front, or opens it again (with the options of
     * extra.nativella.window in composer.json): closing it leaves the
     * application running in the tray.
     */
    public function show(): void
    {
        $this->window->open(self::LABEL);
    }
}
