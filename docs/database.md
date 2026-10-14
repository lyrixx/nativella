# Database

A desktop application keeps its data on the user's machine. Nativella gives each application a [data directory](configuration.md#the-data-directory), and the natural database to put there is SQLite: a file, no server, and a PHP extension (`pdo_sqlite`) that the PHP of Nativella [carries](building.md#the-php-inside).

## Where the database lives

In the data directory, never in the project: the project is replaced by every update of a [standalone build](building.md), the data directory is not. PHP finds it in `$_SERVER['NATIVELLA_DATA_DIR']`, or with `Nativella\Runtime::dataDir()`, which returns null outside Nativella.

| | Database file | Outside Nativella |
|---|---|---|
| [Symfony](symfony.md#the-database) | Where `DATABASE_URL` says: `${NATIVELLA_DATA_DIR}/app.db` | `var/app.db`, with the fallback of the URL |
| [Laravel](laravel.md#the-database) | `database.sqlite` in the data directory, set by the package | The database configuration of the application |
| [Plain PHP](plain-php.md#the-database) | Yours to choose, from `Runtime::dataDir()` | Yours to choose |

To start from an empty database, or to look at a copy, move the data directory:

```bash
NATIVELLA_DATA_DIR=$PWD/var/data nativella run .
```

## Migrations run at boot

There is no deployment step: the user installs a new version and starts it. So the schema follows the code when the application starts, on [`AppBooted`](lifecycle.md#booting), which the runtime handles before it serves any page.

- **Symfony**: with `doctrine/doctrine-migrations-bundle`, the bundle runs the pending Doctrine migrations, all or nothing, before the listeners of the application.
- **Laravel**: the package runs `migrate --force` before the listeners of the application.
- **Plain PHP**: migrate in your `AppBooted` handler.

Write migrations that work on existing data: a new column needs a default value or must be nullable, for the rows already there. And never edit a migration that shipped: some users already ran it.

A failing migration makes the `app.booted` request fail. The runtime logs it (`[event] PHP answered 500 to the "app.booted" event.`) and opens the main window anyway, so show the error in the application rather than counting on the log.

## Plain PHP: migrating with DBAL

DBAL can compare the schema of the database with the one your code describes, and apply the difference. Describe the tables once, and migrate on `AppBooted`:

```php title="src/DatabaseSchema.php"
namespace App;

use Doctrine\DBAL\Connection;
use Doctrine\DBAL\Schema\Column;
use Doctrine\DBAL\Schema\PrimaryKeyConstraint;
use Doctrine\DBAL\Schema\Schema;
use Doctrine\DBAL\Schema\Table;
use Doctrine\DBAL\Types\Types;

final readonly class DatabaseSchema
{
    public function __construct(
        private Connection $connection,
    ) {
    }

    public function migrate(): void
    {
        $snippets = Table::editor()
            ->setUnquotedName('snippet')
            ->setColumns(
                Column::editor()->setUnquotedName('id')->setTypeName(Types::INTEGER)->setAutoincrement(true)->create(),
                Column::editor()->setUnquotedName('title')->setTypeName(Types::STRING)->setLength(200)->create(),
                Column::editor()->setUnquotedName('content')->setTypeName(Types::TEXT)->create(),
                Column::editor()->setUnquotedName('created_at')->setTypeName(Types::DATETIME_IMMUTABLE)->create(),
            )
            ->setPrimaryKeyConstraint(PrimaryKeyConstraint::editor()->setUnquotedColumnNames('id')->create())
            ->create();

        $this->connection->createSchemaManager()->migrateSchema(Schema::editor()->setTables($snippets)->create());
    }
}
```

Changing the definition is enough: on the next launch, `migrateSchema()` creates what is missing and alters what changed. The [Snippets demo](../demos/vanilla) works this way, with DBAL 4.

With PDO alone, keep a version number in `PRAGMA user_version` and run the steps above it:

```php
$steps = [
    1 => 'CREATE TABLE snippet (id INTEGER PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL)',
    2 => 'ALTER TABLE snippet ADD COLUMN created_at TEXT',
];

$version = (int) $pdo->query('PRAGMA user_version')->fetchColumn();
foreach ($steps as $step => $sql) {
    if ($step > $version) {
        $pdo->exec($sql);
        $pdo->exec('PRAGMA user_version = '.$step);
    }
}
```

## Concurrency

Requests are served one at a time by a single PHP thread, so the application never writes to its database from two requests at once. Other processes can: a `bin/console` or `php artisan tinker` pointed at the same data directory. SQLite handles that with locks; keep such sessions short while the application runs.

## Settings are not data

For a handful of preferences (a theme, the last folder opened), the [settings](settings.md) store is simpler than a table. Anything you would list, search or migrate belongs in the database.
