<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Support\Facades\Event;
use Illuminate\Support\Facades\Route;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\NativeEvent;

final class NativeEventEndpointTest extends TestCase
{
    /** @var list<object> */
    private array $dispatched = [];

    protected function setUp(): void
    {
        parent::setUp();

        Event::listen([MenuItemClicked::class, NativeEvent::class], function (object $event): void {
            $this->dispatched[] = $event;
        });
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_INTERNAL']);

        parent::tearDown();
    }

    public function testTheRuntimeDispatchesNativeEvents(): void
    {
        $this->sentByTheRuntime();

        $this->postJson('/_nativella/event', ['name' => 'menu.itemClicked', 'payload' => ['id' => 'notes.new', 'checked' => null]])
            ->assertNoContent();

        $this->assertEquals([new MenuItemClicked('notes.new', null)], $this->dispatched);
    }

    public function testAnEventUnknownToThePackageIsStillDispatched(): void
    {
        $this->sentByTheRuntime();

        $this->postJson('/_nativella/event', ['name' => 'power.suspended', 'payload' => ['battery' => 12]])
            ->assertNoContent();

        $this->assertEquals([new NativeEvent('power.suspended', ['battery' => 12])], $this->dispatched);
    }

    public function testAPageCannotSendNativeEvents(): void
    {
        $this->postJson('/_nativella/event', ['name' => 'menu.itemClicked', 'payload' => ['id' => 'notes.new', 'checked' => null]], ['NATIVELLA_INTERNAL' => '1'])
            ->assertNotFound();

        $this->assertSame([], $this->dispatched);
    }

    public function testTheEndpointNeedsNeitherSessionNorCsrfToken(): void
    {
        $route = Route::getRoutes()->getByName('nativella.event');
        $this->assertNotNull($route);

        $this->assertSame(['POST'], $route->methods());
        $this->assertSame('_nativella/event', $route->uri());
        $this->assertSame([], $route->gatherMiddleware());
    }

    /**
     * What the runtime does for its own requests. Tests requests do not fill
     * $_SERVER, which Nativella\Runtime reads.
     */
    private function sentByTheRuntime(): void
    {
        $_SERVER['NATIVELLA_INTERNAL'] = '1';
    }
}
