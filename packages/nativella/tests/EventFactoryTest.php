<?php

namespace Nativella\Tests;

use Nativella\Event\AppBooted;
use Nativella\Event\EventFactory;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\NativeEvent;
use Nativella\Event\TimerFired;
use Nativella\Event\WindowDownloaded;
use Nativella\Event\WindowDragEntered;
use Nativella\Event\WindowThemeChanged;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\System\Theme;
use Nativella\Tests\Fixtures\RequestBodyStream;
use PHPUnit\Framework\Attributes\BackupGlobals;
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
        $this->expectException(UnexpectedResponseException::class);

        EventFactory::fromJson('{"name":"app.booted"}');
    }

    #[BackupGlobals(true)]
    public function testFromGlobalsIgnoresRequestsFromPages(): void
    {
        $_SERVER['REQUEST_URI'] = '/_nativella/event';
        $_SERVER['REQUEST_METHOD'] = 'POST';
        unset($_SERVER['NATIVELLA_INTERNAL']);

        $this->assertNull(EventFactory::fromGlobals());
    }

    public function testItRejectsABodyThatIsNotJson(): void
    {
        $this->expectException(UnexpectedResponseException::class);

        EventFactory::fromJson('{"name":');
    }

    public function testAThemeThePackageDoesNotKnowIsRefused(): void
    {
        $this->expectException(UnexpectedResponseException::class);
        $this->expectExceptionMessage('The runtime sent a theme the package does not know: "sepia".');

        EventFactory::create('window.themeChanged', ['window' => 'main', 'theme' => 'sepia']);
    }

    public function testEveryEventAlsoComesAsANativeEvent(): void
    {
        $this->assertEquals(
            [new MenuItemClicked('new', null), new NativeEvent('menu.itemClicked', ['id' => 'new', 'checked' => null])],
            EventFactory::allFromJson('{"name":"menu.itemClicked","payload":{"id":"new","checked":null}}'),
        );
        $this->assertEquals(
            [new NativeEvent('power.suspended', ['battery' => 12])],
            EventFactory::allFromJson('{"name":"power.suspended","payload":{"battery":12}}'),
        );
    }

    #[BackupGlobals(true)]
    public function testFromGlobalsReadsTheEventTheRuntimeSent(): void
    {
        $_SERVER['REQUEST_URI'] = '/_nativella/event';
        $_SERVER['REQUEST_METHOD'] = 'POST';
        $_SERVER['NATIVELLA_INTERNAL'] = '1';

        RequestBodyStream::$body = '{"name":"timer.fired","payload":{"name":"sync"}}';
        stream_wrapper_unregister('php');
        stream_wrapper_register('php', RequestBodyStream::class);
        try {
            $event = EventFactory::fromGlobals();
        } finally {
            stream_wrapper_restore('php');
        }

        $this->assertEquals(new TimerFired('sync'), $event);
    }
}
