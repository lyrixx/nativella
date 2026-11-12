<?php

namespace Nativella\Symfony\Tests\Fixtures;

use Nativella\Event\MenuItemClicked;
use Nativella\Event\NativeEvent;

final class RecordingListener
{
    /** @var list<MenuItemClicked|NativeEvent> */
    public array $events = [];

    public function __invoke(MenuItemClicked|NativeEvent $event): void
    {
        $this->events[] = $event;
    }
}
