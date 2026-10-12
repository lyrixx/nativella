<?php

namespace Nativella\Symfony\Tests;

use Nativella\Client;
use Nativella\Client\FakeClient;
use Nativella\Event\MenuItemClicked;
use Nativella\Symfony\Tests\Fixtures\RecordingListener;
use Nativella\Symfony\Tests\Fixtures\TestKernel;
use Nativella\Window;
use PHPUnit\Framework\TestCase;
use Symfony\Component\DependencyInjection\ContainerInterface;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

final class NativellaBundleTest extends TestCase
{
    private TestKernel $kernel;

    protected function setUp(): void
    {
        $this->kernel = new TestKernel('test', true);
        $this->kernel->boot();
    }

    protected function tearDown(): void
    {
        $this->kernel->shutdown();
    }

    public function testTheApisUseTheFakeClient(): void
    {
        $this->service(Window::class, 'test.'.Window::class)->setTitle('Hello');

        $this->assertInstanceOf(FakeClient::class, $this->service(Client::class));
        $this->assertSame([['method' => 'window.setTitle', 'payload' => ['title' => 'Hello', 'window' => null]]], $this->service(FakeClient::class)->calls());
    }

    public function testAnInternalRequestDispatchesTheTypedEvent(): void
    {
        $response = $this->kernel->handle(self::eventRequest(internal: true));

        $this->assertSame(204, $response->getStatusCode());
        $this->assertEquals([new MenuItemClicked('new', null)], $this->service(RecordingListener::class)->events);
    }

    public function testAPageCannotSendNativeEvents(): void
    {
        $this->expectException(NotFoundHttpException::class);

        $this->kernel->handle(self::eventRequest(internal: false), catch: false);
    }

    /**
     * A service of the test container, private ones included.
     *
     * @template T of object
     *
     * @param class-string<T> $class
     *
     * @return T
     */
    private function service(string $class, ?string $id = null): object
    {
        $container = $this->kernel->getContainer()->get('test.service_container');
        $this->assertInstanceOf(ContainerInterface::class, $container);

        $service = $container->get($id ?? $class);
        $this->assertInstanceOf($class, $service);

        return $service;
    }

    private static function eventRequest(bool $internal): Request
    {
        return Request::create(
            '/_nativella/event',
            'POST',
            server: $internal ? ['NATIVELLA_INTERNAL' => '1'] : [],
            content: '{"name":"menu.itemClicked","payload":{"id":"new","checked":null}}',
        );
    }
}
