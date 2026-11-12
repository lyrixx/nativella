<?php

namespace Nativella\Symfony\Tests\Fixtures;

use Nativella\Event\AppBooted;
use Nativella\Window;

final readonly class BootListener
{
    public function __construct(
        private Window $window,
    ) {
    }

    public function __invoke(AppBooted $event): void
    {
        $this->window->setTitle('Notes');
    }
}
