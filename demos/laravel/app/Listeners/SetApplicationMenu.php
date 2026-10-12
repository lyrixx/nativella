<?php

namespace App\Listeners;

use App\Desktop\ApplicationMenu;
use Nativella\Event\AppBooted;

final readonly class SetApplicationMenu
{
    public function __construct(
        private ApplicationMenu $menu,
    ) {
    }

    public function handle(AppBooted $event): void
    {
        $this->menu->set();
    }
}
