<?php

namespace Symfony\Component\DependencyInjection\Loader\Configurator;

use Nativella\Symfony\Twig\NativellaExtension;

return static function (ContainerConfigurator $container): void {
    $container->services()
        ->set(NativellaExtension::class)
        ->tag('twig.extension');
};
