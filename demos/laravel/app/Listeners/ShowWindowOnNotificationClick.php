<?php

namespace App\Listeners;

use App\Desktop\MainWindow;
use Nativella\Event\NotificationClicked;

/**
 * A click on a notification of the application brings its window back.
 */
final readonly class ShowWindowOnNotificationClick
{
    public function __construct(
        private MainWindow $mainWindow,
    ) {
    }

    public function handle(NotificationClicked $event): void
    {
        $this->mainWindow->show();
    }
}
