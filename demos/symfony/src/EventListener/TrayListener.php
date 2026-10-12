<?php

namespace App\EventListener;

use App\Native\Desktop;
use Nativella\Event\TrayClicked;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * A left click on the tray icon shows the window (macOS and Windows; Linux
 * only has the tray menu).
 */
#[AsEventListener]
final readonly class TrayListener
{
    public function __construct(
        private Desktop $desktop,
    ) {
    }

    public function __invoke(TrayClicked $event): void
    {
        if ('left' === $event->button) {
            $this->desktop->showMainWindow();
        }
    }
}
