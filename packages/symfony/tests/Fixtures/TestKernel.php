<?php

namespace Nativella\Symfony\Tests\Fixtures;

use Nativella\Event\MenuItemClicked;
use Nativella\Symfony\NativellaBundle;
use Nativella\Window;
use Symfony\Bundle\FrameworkBundle\FrameworkBundle;
use Symfony\Bundle\FrameworkBundle\Kernel\MicroKernelTrait;
use Symfony\Component\DependencyInjection\Loader\Configurator\ContainerConfigurator;
use Symfony\Component\HttpKernel\Kernel;

final class TestKernel extends Kernel
{
    use MicroKernelTrait;

    /**
     * @return list<FrameworkBundle|NativellaBundle>
     */
    public function registerBundles(): iterable
    {
        return [new FrameworkBundle(), new NativellaBundle()];
    }

    public function getProjectDir(): string
    {
        return __DIR__;
    }

    public function getCacheDir(): string
    {
        return \dirname(__DIR__, 2).'/var/cache/'.$this->environment;
    }

    public function getLogDir(): string
    {
        return \dirname(__DIR__, 2).'/var/log';
    }

    protected function configureContainer(ContainerConfigurator $container): void
    {
        $container->extension('framework', ['test' => true, 'secret' => 'test', 'http_method_override' => false]);
        $container->extension('nativella', ['fake_client' => true]);

        $services = $container->services();
        $services->set(RecordingListener::class)->public()
            ->tag('kernel.event_listener', ['event' => MenuItemClicked::class]);
        // Unused services are removed from the container.
        $services->alias('test.'.Window::class, Window::class)->public();
    }
}
