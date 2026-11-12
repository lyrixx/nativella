<?php

namespace Nativella\Symfony\Tests\Fixtures;

use Doctrine\Bundle\DoctrineBundle\DoctrineBundle;
use Doctrine\Bundle\MigrationsBundle\DoctrineMigrationsBundle;
use Nativella\Event\AppBooted;
use Nativella\Symfony\NativellaBundle;
use Nativella\Window;
use Symfony\Bundle\FrameworkBundle\FrameworkBundle;
use Symfony\Bundle\FrameworkBundle\Kernel\MicroKernelTrait;
use Symfony\Bundle\TwigBundle\TwigBundle;
use Symfony\Bundle\WebProfilerBundle\WebProfilerBundle;
use Symfony\Component\DependencyInjection\Loader\Configurator\ContainerConfigurator;
use Symfony\Component\HttpKernel\Kernel;

use function Symfony\Component\DependencyInjection\Loader\Configurator\service;

/**
 * As an application in development: with the profiler, Twig and the Doctrine
 * migrations, which the bundle only configures when they are there.
 */
final class DevelopmentKernel extends Kernel
{
    use MicroKernelTrait;

    /**
     * @return list<DoctrineBundle|DoctrineMigrationsBundle|FrameworkBundle|NativellaBundle|TwigBundle|WebProfilerBundle>
     */
    public function registerBundles(): iterable
    {
        return [new FrameworkBundle(), new TwigBundle(), new WebProfilerBundle(), new DoctrineBundle(), new DoctrineMigrationsBundle(), new NativellaBundle()];
    }

    public function getProjectDir(): string
    {
        return __DIR__;
    }

    public function getCacheDir(): string
    {
        return \dirname(__DIR__, 2).'/var/cache/development';
    }

    public function getLogDir(): string
    {
        return \dirname(__DIR__, 2).'/var/log';
    }

    protected function configureContainer(ContainerConfigurator $container): void
    {
        $container->extension('framework', [
            'http_method_override' => false,
            'profiler' => ['collect' => true],
            'secret' => 'test',
            'test' => true,
        ]);
        $container->extension('twig', ['default_path' => __DIR__]);
        $container->extension('web_profiler', ['toolbar' => false]);
        $container->extension('doctrine', ['dbal' => ['url' => 'sqlite:///:memory:']]);
        $container->extension('doctrine_migrations', [
            'migrations_paths' => ['Nativella\Symfony\Tests\Fixtures\Migrations' => __DIR__.'/Migrations'],
        ]);
        $container->extension('nativella', ['fake_client' => true]);

        $container->services()
            ->set(BootListener::class)
            ->args([service(Window::class)])
            ->tag('kernel.event_listener', ['event' => AppBooted::class]);
    }
}
