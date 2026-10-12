<?php

namespace Nativella\Symfony\Tests\Fixtures\Migrations;

use Doctrine\DBAL\Schema\Schema;
use Doctrine\Migrations\AbstractMigration;

final class Version1CreateNotes extends AbstractMigration
{
    public function up(Schema $schema): void
    {
        $this->addSql('CREATE TABLE note (id INTEGER PRIMARY KEY, title VARCHAR(200) NOT NULL)');
    }
}
