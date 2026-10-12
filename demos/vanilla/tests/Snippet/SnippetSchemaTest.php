<?php

namespace App\Tests\Snippet;

use App\Snippet\SnippetSchema;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\DriverManager;
use PHPUnit\Framework\TestCase;

final class SnippetSchemaTest extends TestCase
{
    private Connection $connection;
    private SnippetSchema $schema;

    protected function setUp(): void
    {
        $this->connection = DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true]);
        $this->schema = new SnippetSchema($this->connection);
    }

    public function testMigrateCreatesTheTable(): void
    {
        $this->schema->migrate();

        $table = $this->connection->createSchemaManager()->introspectTableByUnquotedName(SnippetSchema::TABLE);
        foreach (['id', 'title', 'content', 'created_at'] as $column) {
            $this->assertTrue($table->hasColumn($column), $column);
        }
        $this->assertTrue($table->getColumn('id')->getAutoincrement());
        $this->assertTrue($table->hasIndex('snippet_created_at'));
    }

    public function testTheDatabaseMatchesTheDefinitionOnceMigrated(): void
    {
        $this->schema->migrate();

        $schemaManager = $this->connection->createSchemaManager();
        $diff = $schemaManager->createComparator()->compareSchemas($schemaManager->introspectSchema(), $this->schema->definition());

        $this->assertTrue($diff->isEmpty());
    }

    public function testMigrateTwiceChangesNothing(): void
    {
        $this->schema->migrate();
        $this->connection->insert(SnippetSchema::TABLE, ['title' => 'Kept', 'content' => 'Still here', 'created_at' => '2026-01-01 00:00:00']);

        $this->schema->migrate();

        $this->assertSame(1, (int) $this->connection->fetchOne('SELECT COUNT(*) FROM snippet'));
    }

    public function testMigrateUpgradesATableOfAnOlderVersionAndKeepsItsRows(): void
    {
        // An older version: shorter titles, no index.
        $this->connection->executeStatement('CREATE TABLE snippet (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, title VARCHAR(100) NOT NULL, content CLOB NOT NULL, created_at DATETIME NOT NULL)');
        $this->connection->insert(SnippetSchema::TABLE, ['title' => 'Old', 'content' => 'From version 1', 'created_at' => '2025-06-01 12:00:00']);

        $this->schema->migrate();

        $table = $this->connection->createSchemaManager()->introspectTableByUnquotedName(SnippetSchema::TABLE);
        $this->assertSame(255, $table->getColumn('title')->getLength());
        $this->assertTrue($table->hasIndex('snippet_created_at'));
        $this->assertSame(
            ['id' => 1, 'title' => 'Old', 'content' => 'From version 1', 'created_at' => '2025-06-01 12:00:00'],
            $this->connection->fetchAssociative('SELECT id, title, content, created_at FROM snippet'),
        );
    }

    public function testMigrateIfMissingCreatesAMissingTable(): void
    {
        $this->schema->migrateIfMissing();

        $this->assertTrue($this->connection->createSchemaManager()->tableExists(SnippetSchema::TABLE));
    }

    public function testMigrateIfMissingLeavesAnExistingTableAlone(): void
    {
        $this->connection->executeStatement('CREATE TABLE snippet (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, title VARCHAR(100) NOT NULL, content CLOB NOT NULL, created_at DATETIME NOT NULL)');

        $this->schema->migrateIfMissing();

        $table = $this->connection->createSchemaManager()->introspectTableByUnquotedName(SnippetSchema::TABLE);
        $this->assertFalse($table->hasIndex('snippet_created_at'));
    }
}
