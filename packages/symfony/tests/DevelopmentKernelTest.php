<?php

namespace Nativella\Symfony\Tests;

use Doctrine\DBAL\Connection;
use Nativella\Client;
use Nativella\Client\FakeClient;
use Nativella\Symfony\Client\TraceableClient;
use Nativella\Symfony\DataCollector\NativellaDataCollector;
use Nativella\Symfony\Tests\Fixtures\DevelopmentKernel;
use PHPUnit\Framework\TestCase;
use Symfony\Component\DependencyInjection\ContainerInterface;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpKernel\Profiler\Profiler;
use Twig\Environment;

/**
 * What the bundle configures only next to other bundles: the profiler, Twig
 * and the Doctrine migrations.
 */
final class DevelopmentKernelTest extends TestCase
{
    private DevelopmentKernel $kernel;

    protected function setUp(): void
    {
        $this->kernel = new DevelopmentKernel('test', true);
        $this->kernel->boot();
    }

    protected function tearDown(): void
    {
        $this->kernel->shutdown();
    }

    public function testTheProfilerTracesTheCallsOfTheClient(): void
    {
        $this->assertInstanceOf(TraceableClient::class, $this->service(Client::class));

        $request = self::booted();
        $response = $this->kernel->handle($request);
        $this->assertSame(204, $response->getStatusCode());
        // Where the profiler saves the profile.
        $this->kernel->terminate($request, $response);

        $profile = $this->service(Profiler::class, 'profiler')->loadProfileFromResponse($response);
        $this->assertNotNull($profile);
        $collector = $profile->getCollector(NativellaDataCollector::class);
        $this->assertInstanceOf(NativellaDataCollector::class, $collector);
        $this->assertSame(1, $collector->getCallCount());
        $this->assertSame('window.setTitle', $collector->getCalls()[0]['method']);
        $this->assertCount(1, $this->service(FakeClient::class)->calls('window.setTitle'));
    }

    public function testTheDatabaseIsMigratedWhenTheApplicationBoots(): void
    {
        $this->kernel->handle(self::booted());

        $columns = $this->service(Connection::class, 'doctrine.dbal.default_connection')->fetchFirstColumn("SELECT name FROM pragma_table_info('note')");
        $this->assertSame(['id', 'title', 'body'], $columns);
    }

    public function testTheTemplatesKnowWhetherNativeCallsWork(): void
    {
        $template = $this->service(Environment::class, 'twig')->createTemplate('{{ nativella_is_native() ? "native" : "web" }}');

        $this->assertSame('native', $template->render());
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

    private static function booted(): Request
    {
        return Request::create('/_nativella/event', 'POST', server: ['NATIVELLA_INTERNAL' => '1'], content: '{"name":"app.booted","payload":{}}');
    }
}
