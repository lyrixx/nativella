<?php

namespace Symfony\Component\DependencyInjection\Loader\Configurator;

use Nativella\App;
use Nativella\BarcodeScanner;
use Nativella\Biometrics;
use Nativella\ChildProcess;
use Nativella\Client;
use Nativella\Client\ExtensionClient;
use Nativella\Clipboard;
use Nativella\Dialog;
use Nativella\Geolocation;
use Nativella\GlobalShortcut;
use Nativella\Haptics;
use Nativella\Menu;
use Nativella\Nativella;
use Nativella\Notification;
use Nativella\Page;
use Nativella\Power;
use Nativella\Screen;
use Nativella\Secrets;
use Nativella\Settings;
use Nativella\Shell;
use Nativella\Symfony\EventListener\NativeEventListener;
use Nativella\System;
use Nativella\Timer;
use Nativella\Tray;
use Nativella\Window;
use Symfony\Component\HttpKernel\KernelEvents;

return static function (ContainerConfigurator $container): void {
    $services = $container->services();

    $services->set(ExtensionClient::class);
    $services->alias(Client::class, ExtensionClient::class);

    foreach ([App::class, BarcodeScanner::class, Biometrics::class, ChildProcess::class, Clipboard::class, Dialog::class, Geolocation::class, GlobalShortcut::class, Haptics::class, Menu::class, Nativella::class, Notification::class, Page::class, Power::class, Screen::class, Secrets::class, Settings::class, Shell::class, System::class, Timer::class, Tray::class, Window::class] as $api) {
        $services->set($api)->args([service(Client::class)]);
    }

    // Before the router and the firewall: native events need neither.
    $services->set(NativeEventListener::class)
        ->args([service('event_dispatcher')])
        ->tag('kernel.event_listener', ['event' => KernelEvents::REQUEST, 'priority' => 64]);
};
