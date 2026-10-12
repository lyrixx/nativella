<?php

namespace Nativella\Symfony;

use Symfony\Component\Config\Definition\Configurator\DefinitionConfigurator;
use Symfony\Component\DependencyInjection\ContainerBuilder;
use Symfony\Component\DependencyInjection\Loader\Configurator\ContainerConfigurator;
use Symfony\Component\HttpKernel\Bundle\AbstractBundle;

/**
 * Runs the application in Nativella: the native APIs as services, native
 * events dispatched as Symfony events, the database migrated at boot.
 */
final class NativellaBundle extends AbstractBundle
{
    public function getPath(): string
    {
        return \dirname(__DIR__);
    }

    public function configure(DefinitionConfigurator $definition): void
    {
        $definition->rootNode()
            ->children()
                ->booleanNode('fake_client')
                    ->info('Answers native calls with Nativella\Client\FakeClient, to test the application outside Nativella.')
                    ->defaultFalse()
                ->end()
                ->booleanNode('migrate_on_boot')
                    ->info('Runs the Doctrine migrations when the application starts (Nativella\Event\AppBooted).')
                    ->defaultTrue()
                ->end()
            ->end();
    }

    /**
     * @param array{fake_client: bool, migrate_on_boot: bool} $config
     */
    public function loadExtension(array $config, ContainerConfigurator $container, ContainerBuilder $builder): void
    {
        $container->import('../config/services.php');

        /** @var array<string, class-string> $bundles */
        $bundles = $builder->getParameter('kernel.bundles');

        if ($config['fake_client']) {
            $container->import('../config/fake_client.php');
        }
        if ($config['migrate_on_boot'] && isset($bundles['DoctrineMigrationsBundle'])) {
            $container->import('../config/migrations.php');
        }
        if (isset($bundles['TwigBundle'])) {
            $container->import('../config/twig.php');
        }
        if (isset($bundles['WebProfilerBundle']) && $builder->getParameter('kernel.debug')) {
            $container->import('../config/profiler.php');
        }
    }
}
