<?php

namespace App\Listeners;

use App\Desktop\TaskTray;
use Nativella\Event\AppBooted;

final readonly class ShowTrayIcon
{
    public function __construct(
        private TaskTray $tray,
    ) {
    }

    public function handle(AppBooted $event): void
    {
        $this->tray->refresh();
    }
}
