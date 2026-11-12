<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\WindowThemeChanged;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\System;
use Nativella\System\SystemInfo;
use Nativella\System\Theme;
use PHPUnit\Framework\TestCase;

final class SystemTest extends TestCase
{
    public function testTheOperatingSystem(): void
    {
        $client = new FakeClient();
        $client->respond('system.info', ['platform' => 'macos', 'family' => 'unix', 'arch' => 'aarch64', 'version' => '15.1', 'hostname' => null, 'locale' => 'fr-FR']);

        $this->assertEquals(new SystemInfo('macos', 'unix', 'aarch64', '15.1', null, 'fr-FR'), new System($client)->info());
    }

    public function testTheThemeOfAWindow(): void
    {
        $client = new FakeClient();
        $client->respond('system.theme', ['theme' => 'dark']);

        $this->assertSame(Theme::Dark, new System($client)->theme('settings'));
        $this->assertSame(['window' => 'settings'], $client->calls()[0]->payload);
    }

    public function testTheThemeChanges(): void
    {
        $this->assertEquals(new WindowThemeChanged('main', Theme::Light), EventFactory::create('window.themeChanged', ['window' => 'main', 'theme' => 'light']));
    }

    public function testAThemeThePackageDoesNotKnowIsRefused(): void
    {
        $client = new FakeClient();
        $client->respond('system.theme', ['theme' => 'sepia']);

        $this->expectException(UnexpectedResponseException::class);

        new System($client)->theme();
    }
}
