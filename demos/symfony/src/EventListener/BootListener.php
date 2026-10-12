<?php

namespace App\EventListener;

use App\Native\Desktop;
use Nativella\Event\AppBooted;
use Nativella\GlobalShortcut;
use Nativella\Runtime;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * Sets the application up when it starts. The bundle has migrated the
 * database already.
 */
#[AsEventListener]
final readonly class BootListener
{
    public function __construct(
        private Desktop $desktop,
        private GlobalShortcut $globalShortcut,
    ) {
    }

    public function __invoke(AppBooted $event): void
    {
        // Phones have no menu bar, tray icon or global shortcut.
        if (!Runtime::isMobile()) {
            $this->desktop->installMenu();
            $this->desktop->refreshTray();
            $this->globalShortcut->register(Desktop::QUICK_NOTE_SHORTCUT);
        }
        $this->desktop->openMainWindow();
    }
}
