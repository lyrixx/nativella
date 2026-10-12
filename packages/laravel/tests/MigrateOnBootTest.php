<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Foundation\Application;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Nativella\Event\AppBooted;
use Orchestra\Testbench\Attributes\DefineEnvironment;

final class MigrateOnBootTest extends TestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        // As a package would, without running them.
        app('migrator')->path(__DIR__.'/Fixtures/migrations');
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_INTERNAL']);

        parent::tearDown();
    }

    public function testTheDatabaseIsMigratedWhenTheApplicationStarts(): void
    {
        event(new AppBooted());

        $this->assertTrue(Schema::hasTable('notes'));
    }

    public function testTheRuntimeStartsTheApplicationThroughTheEndpoint(): void
    {
        $_SERVER['NATIVELLA_INTERNAL'] = '1';

        $this->postJson('/_nativella/event', ['name' => 'app.booted', 'payload' => []])->assertNoContent();

        $this->assertTrue(Schema::hasTable('notes'));
    }

    #[DefineEnvironment('disableMigrations')]
    public function testTheConfigurationCanDisableTheMigrations(): void
    {
        event(new AppBooted());

        $this->assertFalse(Schema::hasTable('notes'));
    }

    public function testASchemaDumpIsLoadedWithoutTheSqlite3Command(): void
    {
        $dump = database_path('schema/'.DB::connection()->getName().'-schema.sql');
        @mkdir(\dirname($dump), recursive: true);
        // As `schema:dump` writes it: the tables, and the migrations already run.
        file_put_contents($dump, "CREATE TABLE IF NOT EXISTS \"migrations\" (\"id\" integer primary key autoincrement not null, \"migration\" varchar not null, \"batch\" integer not null);\nCREATE TABLE \"dumped\" (\"id\" integer);\nINSERT INTO migrations VALUES(1,'2020_01_01_000000_create_dumped_table',1);\n");

        try {
            event(new AppBooted());
        } finally {
            unlink($dump);
        }

        $this->assertTrue(Schema::hasTable('dumped'));
        $this->assertTrue(Schema::hasTable('notes'), 'The migrations run after the dump.');
    }

    protected function disableMigrations(Application $app): void
    {
        $app['config']->set('nativella.migrate_on_boot', false);
    }
}
