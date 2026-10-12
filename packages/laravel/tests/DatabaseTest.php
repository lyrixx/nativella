<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Foundation\Application;
use Illuminate\Support\Facades\DB;
use Orchestra\Testbench\Attributes\DefineEnvironment;

final class DatabaseTest extends TestCase
{
    private string $dataDir;

    protected function setUp(): void
    {
        // What the runtime sets for every request.
        $this->dataDir = \dirname(__DIR__).'/var/data-'.bin2hex(random_bytes(4));
        mkdir($this->dataDir, recursive: true);
        $_SERVER['NATIVELLA_DATA_DIR'] = $this->dataDir;

        parent::setUp();
    }

    protected function tearDown(): void
    {
        parent::tearDown();

        unset($_SERVER['NATIVELLA_DATA_DIR']);
        array_map(unlink(...), glob($this->dataDir.'/*') ?: []);
        rmdir($this->dataDir);
    }

    public function testTheDatabaseLivesInTheDataDirectory(): void
    {
        $database = $this->dataDir.\DIRECTORY_SEPARATOR.'database.sqlite';

        $this->assertSame('sqlite', config('database.default'));
        $this->assertSame('sqlite', config('queue.batching.database'));
        $this->assertSame('sqlite', config('queue.failed.database'));
        $this->assertSame($database, DB::connection()->getDatabaseName());
        $this->assertFileExists($database);

        DB::statement('CREATE TABLE notes (title TEXT)');
        DB::table('notes')->insert(['title' => 'Groceries']);
        $this->assertSame(1, DB::table('notes')->count());
    }

    public function testAnExistingDatabaseIsKept(): void
    {
        DB::statement('CREATE TABLE notes (title TEXT)');
        DB::table('notes')->insert(['title' => 'Groceries']);

        $this->refreshApplication();

        $this->assertSame(1, DB::table('notes')->count());
    }

    public function testTheUrlOfAWebDeploymentIsIgnored(): void
    {
        $this->assertNull(config('database.connections.sqlite.url'));
        $this->assertSame('DEFERRED', config('database.connections.sqlite.transaction_mode'));
    }

    #[DefineEnvironment('keepTheDatabaseConfiguration')]
    public function testTheConfigurationCanKeepTheDatabaseWhereItIs(): void
    {
        $this->assertSame('testing', config('database.default'));
        $this->assertFileDoesNotExist($this->dataDir.'/database.sqlite');
    }

    protected function keepTheDatabaseConfiguration(Application $app): void
    {
        $app['config']->set('nativella.database', false);
    }
}
