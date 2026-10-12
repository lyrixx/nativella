<?php

namespace Nativella\Symfony\Tests\Fixtures;

use Nativella\Event\MenuItemClicked;

final class RecordingListener
{
    /** @var list<MenuItemClicked> */
    public array $events = [];

    public function __invoke(MenuItemClicked $event): void
    {
        $this->events[] = $event;
    }
}
