<?php

namespace App\Snippet;

use Doctrine\DBAL\Connection;
use Doctrine\DBAL\Schema\Column;
use Doctrine\DBAL\Schema\Index;
use Doctrine\DBAL\Schema\PrimaryKeyConstraint;
use Doctrine\DBAL\Schema\Schema;
use Doctrine\DBAL\Schema\Table;
use Doctrine\DBAL\Types\Types;

/**
 * The tables of the application, created and upgraded by the schema tools of
 * DBAL: the comparator turns the difference between the database and this
 * definition into SQL. Changing the definition is enough to upgrade the
 * database of every user on their next launch; a new column needs a default
 * value or to be nullable, for the rows already there.
 */
final readonly class SnippetSchema
{
    public const string TABLE = 'snippet';

    public function __construct(
        private Connection $connection,
    ) {
    }

    public function definition(): Schema
    {
        $table = Table::editor()
            ->setUnquotedName(self::TABLE)
            ->setColumns(
                Column::editor()->setUnquotedName('content')->setTypeName(Types::TEXT)->create(),
                Column::editor()->setUnquotedName('created_at')->setTypeName(Types::DATETIME_IMMUTABLE)->create(),
                Column::editor()->setUnquotedName('id')->setTypeName(Types::INTEGER)->setAutoincrement(true)->create(),
                Column::editor()->setUnquotedName('title')->setTypeName(Types::STRING)->setLength(SnippetInput::MAX_TITLE_LENGTH)->create(),
            )
            ->setPrimaryKeyConstraint(PrimaryKeyConstraint::editor()->setUnquotedColumnNames('id')->create())
            ->setIndexes(Index::editor()->setUnquotedName('snippet_created_at')->setUnquotedColumnNames('created_at')->create())
            ->create();

        return Schema::editor()->setTables($table)->create();
    }

    /**
     * Brings the database up to date: creates what is missing, alters what
     * changed. Runs when the application boots.
     */
    public function migrate(): void
    {
        $this->connection->createSchemaManager()->migrateSchema($this->definition());
    }

    /**
     * Migrates only when the table is missing: a cheap check for requests no
     * "app.booted" event came before, as under php -S.
     */
    public function migrateIfMissing(): void
    {
        if (!$this->connection->createSchemaManager()->tableExists(self::TABLE)) {
            $this->migrate();
        }
    }
}
