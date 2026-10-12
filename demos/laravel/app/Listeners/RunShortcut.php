<?php

namespace App\Listeners;

use App\Desktop\MainWindow;
use App\Desktop\Shortcut;
use Nativella\Event\GlobalShortcutPressed;

final readonly class RunShortcut
{
    public function __construct(
        private MainWindow $mainWindow,
    ) {
    }

    public function handle(GlobalShortcutPressed $event): void
    {
        // The runtime only reports the shortcuts this application registered.
        match (Shortcut::from($event->accelerator)) {
            Shortcut::ShowWindow => $this->mainWindow->show(),
        };
    }
}
