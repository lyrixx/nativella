<?php

namespace Nativella\Symfony\Tests\Fixtures\Migrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

/**
 * As SQLite table rebuilds are: outside a transaction, where
 * PRAGMA foreign_keys has an effect.
 */
final class Version2RebuildNotes extends AbstractMigration
{
    public function isTransactional(): bool
    {
        return false;
    }

    public function up(Schema $schema): void
    {
        $this->addSql('ALTER TABLE note ADD COLUMN body TEXT');
    }
}
