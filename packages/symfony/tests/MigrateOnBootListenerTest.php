<?php

namespace Nativella\Symfony\Tests;

use Doctrine\DBAL\Connection;
use Doctrine\DBAL\DriverManager;
use Doctrine\Migrations\Configuration\Connection\ExistingConnection;
use Doctrine\Migrations\Configuration\Migration\ConfigurationArray;
use Doctrine\Migrations\DependencyFactory;
use Nativella\Event\AppBooted;
use Nativella\Symfony\EventListener\MigrateOnBootListener;
use PHPUnit\Framework\TestCase;
use Psr\Log\AbstractLogger;

final class MigrateOnBootListenerTest extends TestCase
{
    private Connection $connection;

    protected function setUp(): void
    {
        $this->connection = DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true]);
    }

    public function testANonTransactionalMigrationDoesNotBlockTheOthers(): void
    {
        $this->listener(__DIR__.'/Fixtures/Migrations')(new AppBooted());

        $columns = $this->connection->fetchFirstColumn("SELECT name FROM pragma_table_info('note')");
        $this->assertSame(['id', 'title', 'body'], $columns);
    }

    public function testAMissingDirectoryMeansNoMigration(): void
    {
        $logger = new class extends AbstractLogger {
            /** @var list<string> */
            public array $warnings = [];

            public function log($level, \Stringable|string $message, array $context = []): void
            {
                if ('warning' === $level) {
                    $this->warnings[] = (string) $message;
                }
            }
        };

        $this->listener(__DIR__.'/Fixtures/NoMigrations', $logger)(new AppBooted());

        $this->assertSame(['The migrations directory {directory} does not exist: no migration runs.'], $logger->warnings);
        $this->assertFalse($this->connection->createSchemaManager()->tablesExist(['note']));
    }

    private function listener(string $directory, ?AbstractLogger $logger = null): MigrateOnBootListener
    {
        $configuration = new ConfigurationArray([
            'migrations_paths' => ['Nativella\Symfony\Tests\Fixtures\Migrations' => $directory],
        ]);

        return new MigrateOnBootListener(DependencyFactory::fromConnection($configuration, new ExistingConnection($this->connection)), $logger);
    }
}
