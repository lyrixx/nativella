<?php

namespace App\EventListener;

use App\Native\Desktop;
use Nativella\Event\GlobalShortcutPressed;
use Nativella\Window;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * The global shortcut brings the window to the front, on a new note.
 */
#[AsEventListener]
final readonly class QuickNoteListener
{
    public function __construct(
        private Desktop $desktop,
        private Window $window,
    ) {
    }

    public function __invoke(GlobalShortcutPressed $event): void
    {
        if (Desktop::QUICK_NOTE_SHORTCUT !== $event->accelerator) {
            return;
        }

        $this->desktop->showMainWindow();
        $this->window->navigate('/notes/new', Desktop::MAIN_WINDOW);
    }
}
