<?php

namespace App\EventListener;

use App\Native\Desktop;
use Nativella\Event\AppBooted;
use Nativella\Exception\NativeCallException;
use Nativella\GlobalShortcut;
use Nativella\Runtime;
use Psr\Log\LoggerInterface;
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
        private LoggerInterface $logger,
    ) {
    }

    public function __invoke(AppBooted $event): void
    {
        // Phones have no menu bar, tray icon or global shortcut.
        if (!Runtime::isMobile()) {
            $this->desktop->installMenu();
            $this->desktop->refreshTray();
            try {
                $this->globalShortcut->register(Desktop::QUICK_NOTE_SHORTCUT);
            } catch (NativeCallException $e) {
                // Another application holds it: Notes works without.
                $this->logger->warning('The global shortcut {shortcut} is not available.', ['shortcut' => Desktop::QUICK_NOTE_SHORTCUT, 'exception' => $e]);
            }
        }
        $this->desktop->openMainWindow();
    }
}
