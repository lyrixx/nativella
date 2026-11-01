<?php

namespace Symfony\Component\DependencyInjection\Loader\Configurator;

use Nativella\Client;
use Nativella\Symfony\Twig\NativellaExtension;

return static function (ContainerConfigurator $container): void {
    $container->services()
        ->set(NativellaExtension::class)
        ->args([service(Client::class)])
        ->tag('twig.extension');
};
