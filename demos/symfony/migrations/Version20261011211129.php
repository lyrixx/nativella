<?php

declare(strict_types=1);

namespace DoctrineMigrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version20261011211129 extends AbstractMigration
{
    public function getDescription(): string
    {
        return 'Create the note table.';
    }

    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE note (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, title VARCHAR(200) NOT NULL, body CLOB NOT NULL, pinned BOOLEAN NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL)');
    }

    public function down(Schema $schema): void
    {
        $this->addSql('DROP TABLE note');
    }
}
