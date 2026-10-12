<?php

namespace Symfony\Component\DependencyInjection\Loader\Configurator;

use Nativella\Client;
use Nativella\Client\FakeClient;

return static function (ContainerConfigurator $container): void {
    $services = $container->services();

    // Public, for tests to configure its answers and read the calls.
    $services->set(FakeClient::class)->public();
    $services->alias(Client::class, FakeClient::class);
};
