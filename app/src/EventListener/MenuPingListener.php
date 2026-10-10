<?php

namespace App\EventListener;

use App\Native\Native;
use App\Native\NativeEvent;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * Answers the "Ping PHP" menu item with a desktop notification: a native
 * event goes to PHP, and PHP calls back into native code.
 */
#[AsEventListener(event: 'native.menu.ping')]
final readonly class MenuPingListener
{
    public function __construct(
        private Native $native,
    ) {
    }

    public function __invoke(NativeEvent $event): void
    {
        $this->native->call('notification.show', [
            'title' => 'Pong from PHP',
            'body' => \sprintf('PHP %s handled the "%s" event.', \PHP_VERSION, $event->name),
        ]);
    }
}
