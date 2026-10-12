<?php

namespace Symfony\Component\DependencyInjection\Loader\Configurator;

use Nativella\Event\AppBooted;
use Nativella\Symfony\EventListener\MigrateOnBootListener;

return static function (ContainerConfigurator $container): void {
    $container->services()
        ->set(MigrateOnBootListener::class)
        ->args([
            service('doctrine.migrations.dependency_factory'),
            service('logger')->nullOnInvalid(),
        ])
        // Before the listeners of the application, which may use the database.
        ->tag('kernel.event_listener', ['event' => AppBooted::class, 'priority' => 1024])
        ->tag('monolog.logger', ['channel' => 'nativella']);
};
