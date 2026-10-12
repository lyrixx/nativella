<?php

namespace Symfony\Component\DependencyInjection\Loader\Configurator;

use Nativella\Client;
use Nativella\Symfony\Client\TraceableClient;
use Nativella\Symfony\DataCollector\NativellaDataCollector;

return static function (ContainerConfigurator $container): void {
    $services = $container->services();

    $services->set(TraceableClient::class)
        ->decorate(Client::class)
        ->args([service('.inner')])
        // The calls of a request only: kernels reused between requests (tests,
        // worker runtimes) reset it.
        ->tag('kernel.reset', ['method' => 'reset']);

    // The collector names itself and its template (AbstractDataCollector).
    $services->set(NativellaDataCollector::class)
        ->args([service(TraceableClient::class)])
        ->tag('data_collector', ['priority' => 260]);
};
