<?php

namespace App\Listeners;

use App\Desktop\Shortcut;
use Nativella\Event\AppBooted;
use Nativella\Exception\NativeCallException;
use Nativella\GlobalShortcut;
use Nativella\Runtime;
use Psr\Log\LoggerInterface;

final readonly class RegisterGlobalShortcuts
{
    public function __construct(
        private GlobalShortcut $globalShortcut,
        private LoggerInterface $logger,
    ) {
    }

    public function handle(AppBooted $event): void
    {
        // Phones and tablets have no global shortcuts.
        if (Runtime::isMobile()) {
            return;
        }

        foreach (Shortcut::cases() as $shortcut) {
            try {
                $this->globalShortcut->register($shortcut->value);
            } catch (NativeCallException $e) {
                // Another application holds it: the application works without.
                $this->logger->warning('The global shortcut {shortcut} is not available.', ['shortcut' => $shortcut->value, 'exception' => $e]);
            }
        }
    }
}
