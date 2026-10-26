<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Support\Facades\Blade;
use Illuminate\Support\ServiceProvider;
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
use Nativella\Laravel\Facades;
use Nativella\Laravel\NativellaServiceProvider;
use Nativella\Menu;
use Nativella\Nativella;
use Nativella\Notification;
use Nativella\Page;
use Nativella\Power;
use Nativella\Screen;
use Nativella\Secrets;
use Nativella\Settings;
use Nativella\Shell;
use Nativella\System;
use Nativella\Timer;
use Nativella\Tray;
use Nativella\Window;
use PHPUnit\Framework\Attributes\DataProvider;

final class NativellaServiceProviderTest extends TestCase
{
    public function testTheClientCallsTheRuntime(): void
    {
        $this->assertInstanceOf(ExtensionClient::class, app(Client::class));
        $this->assertSame(app(Client::class), app(Client::class));
    }

    /**
     * @param class-string $api
     */
    #[DataProvider('provideApis')]
    public function testEveryApiIsASingleton(string $api): void
    {
        $this->assertInstanceOf($api, app($api));
        $this->assertSame(app($api), app($api));
    }

    /**
     * @return iterable<string, array{class-string}>
     */
    public static function provideApis(): iterable
    {
        foreach ([App::class, BarcodeScanner::class, Biometrics::class, ChildProcess::class, Clipboard::class, Dialog::class, Geolocation::class, GlobalShortcut::class, Haptics::class, Menu::class, Nativella::class, Notification::class, Page::class, Power::class, Screen::class, Secrets::class, Settings::class, Shell::class, System::class, Timer::class, Tray::class, Window::class] as $api) {
            yield $api => [$api];
        }
    }

    public function testEachFacadeServesItsApi(): void
    {
        $this->assertSame(app(App::class), Facades\App::getFacadeRoot());
        $this->assertSame(app(Clipboard::class), Facades\Clipboard::getFacadeRoot());
        $this->assertSame(app(Dialog::class), Facades\Dialog::getFacadeRoot());
        $this->assertSame(app(GlobalShortcut::class), Facades\GlobalShortcut::getFacadeRoot());
        $this->assertSame(app(Menu::class), Facades\Menu::getFacadeRoot());
        $this->assertSame(app(Nativella::class), Facades\Nativella::getFacadeRoot());
        $this->assertSame(app(Notification::class), Facades\Notification::getFacadeRoot());
        $this->assertSame(app(Page::class), Facades\Page::getFacadeRoot());
        $this->assertSame(app(Screen::class), Facades\Screen::getFacadeRoot());
        $this->assertSame(app(Settings::class), Facades\Settings::getFacadeRoot());
        $this->assertSame(app(Shell::class), Facades\Shell::getFacadeRoot());
        $this->assertSame(app(System::class), Facades\System::getFacadeRoot());
        $this->assertSame(app(Tray::class), Facades\Tray::getFacadeRoot());
        $this->assertSame(app(Window::class), Facades\Window::getFacadeRoot());
    }

    public function testTheConfigurationHasDefaults(): void
    {
        $this->assertTrue(config('nativella.database'));
        $this->assertTrue(config('nativella.migrate_on_boot'));
    }

    public function testTheConfigurationCanBePublished(): void
    {
        $paths = ServiceProvider::pathsToPublish(NativellaServiceProvider::class, 'nativella-config');

        $this->assertCount(1, $paths);
        foreach ($paths as $source => $target) {
            $this->assertFileEquals(\dirname(__DIR__).'/config/nativella.php', $source);
            $this->assertSame(config_path('nativella.php'), $target);
        }
    }

    public function testTheNativeDirectiveHidesNativeFeaturesOutsideNativella(): void
    {
        $this->assertSame('browser', trim(Blade::render('@native desktop @else browser @endnative')));
    }
}
