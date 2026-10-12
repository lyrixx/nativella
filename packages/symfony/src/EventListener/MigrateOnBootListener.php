<?php

namespace Nativella\Symfony\EventListener;

use Doctrine\Migrations\DependencyFactory;
use Doctrine\Migrations\MigratorConfiguration;
use Nativella\Event\AppBooted;
use Psr\Log\LoggerInterface;

/**
 * Migrates the database when the application starts: a desktop application
 * has no deployment step, its database lives on the user's machine.
 */
final readonly class MigrateOnBootListener
{
    public function __construct(
        private DependencyFactory $dependencyFactory,
        private ?LoggerInterface $logger = null,
    ) {
    }

    public function __invoke(AppBooted $event): void
    {
        // Git keeps no empty directory: a project may have none yet.
        foreach ($this->dependencyFactory->getConfiguration()->getMigrationDirectories() as $directory) {
            if (!is_dir($directory)) {
                $this->logger?->warning('The migrations directory {directory} does not exist: no migration runs.', ['directory' => $directory]);

                return;
            }
        }

        $this->dependencyFactory->getMetadataStorage()->ensureInitialized();

        if (0 === \count($this->dependencyFactory->getMigrationRepository()->getMigrations())) {
            return;
        }

        $latest = $this->dependencyFactory->getVersionAliasResolver()->resolveVersionAlias('latest');
        $plan = $this->dependencyFactory->getMigrationPlanCalculator()->getPlanUntilVersion($latest);
        if (0 === \count($plan)) {
            return;
        }

        $this->logger?->info('Migrating the database to {version}.', ['version' => (string) $latest]);
        // As `doctrine:migrations:migrate` does: all or nothing only when
        // configured, since a single non-transactional migration (a table
        // rebuild on SQLite) would refuse to run otherwise.
        $configuration = new MigratorConfiguration()
            ->setAllOrNothing($this->dependencyFactory->getConfiguration()->isAllOrNothing());
        $this->dependencyFactory->getMigrator()->migrate($plan, $configuration);
    }
}
