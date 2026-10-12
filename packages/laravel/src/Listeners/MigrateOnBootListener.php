<?php

namespace Nativella\Laravel\Listeners;

use Illuminate\Container\Attributes\Config;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Database\DatabaseManager;
use Nativella\Event\AppBooted;

/**
 * Migrates the database when the application starts: a desktop application
 * has no deployment step, its database lives on the user's machine.
 */
final readonly class MigrateOnBootListener
{
    public function __construct(
        private Kernel $artisan,
        private DatabaseManager $database,
        #[Config('nativella.migrate_on_boot')]
        private bool $enabled,
        #[Config('database.migrations.table', 'migrations')]
        private string $migrationsTable,
    ) {
    }

    public function handle(AppBooted $event): void
    {
        if (!$this->enabled) {
            return;
        }

        $this->loadSchemaDump();

        // A failing migration throws: the runtime then logs the error status.
        $this->artisan->call('migrate', ['--force' => true]);
    }

    /**
     * On a fresh SQLite database, `migrate` would load the schema dump of
     * `schema:dump` with the sqlite3 command, which users' machines lack:
     * load it through PDO instead, as Laravel does for in-memory databases.
     */
    private function loadSchemaDump(): void
    {
        $connection = $this->database->connection();
        $dump = database_path('schema/'.$connection->getName().'-schema.sql');
        if ('sqlite' !== $connection->getDriverName() || !is_file($dump) || $connection->getSchemaBuilder()->hasTable($this->migrationsTable)) {
            return;
        }

        $sql = file_get_contents($dump);
        if (false === $sql) {
            throw new \RuntimeException(\sprintf('Cannot read the schema dump "%s".', $dump));
        }

        $connection->getPdo()->exec($sql);
    }
}
