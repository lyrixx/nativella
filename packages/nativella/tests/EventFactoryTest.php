<?php

namespace Nativella\Tests;

use Nativella\Event\AppBooted;
use Nativella\Event\EventFactory;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\NativeEvent;
use Nativella\Event\WindowDownloaded;
use Nativella\Event\WindowDragEntered;
use Nativella\Event\WindowThemeChanged;
use Nativella\System\Theme;
use PHPUnit\Framework\TestCase;

final class EventFactoryTest extends TestCase
{
    public function testItCreatesTypedEvents(): void
    {
        $this->assertInstanceOf(AppBooted::class, EventFactory::create('app.booted', []));
        $this->assertEquals(new MenuItemClicked('new', null), EventFactory::fromJson('{"name":"menu.itemClicked","payload":{"id":"new","checked":null}}'));
        $this->assertEquals(new WindowThemeChanged('main', Theme::Dark), EventFactory::create('window.themeChanged', ['window' => 'main', 'theme' => 'dark']));
        $this->assertEquals(new WindowDownloaded('main', '/home/me/Downloads/notes.csv'), EventFactory::create('window.downloaded', ['window' => 'main', 'path' => '/home/me/Downloads/notes.csv']));
        $this->assertEquals(new WindowDragEntered('main', ['/home/me/a.md']), EventFactory::create('window.dragEntered', ['window' => 'main', 'paths' => ['/home/me/a.md']]));
    }

    public function testAnUnknownEventStaysGeneric(): void
    {
        $this->assertEquals(new NativeEvent('power.suspended', ['a' => 1]), EventFactory::create('power.suspended', ['a' => 1]));
    }

    public function testItRejectsAMalformedBody(): void
    {
        $this->expectException(\UnexpectedValueException::class);

        EventFactory::fromJson('{"name":"app.booted"}');
    }

    public function testFromGlobalsIgnoresRequestsFromPages(): void
    {
        $_SERVER['REQUEST_URI'] = '/_nativella/event';
        $_SERVER['REQUEST_METHOD'] = 'POST';
        unset($_SERVER['NATIVELLA_INTERNAL']);

        $this->assertNull(EventFactory::fromGlobals());
    }
}
