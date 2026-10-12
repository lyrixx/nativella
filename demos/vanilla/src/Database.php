<?php

namespace App;

use Doctrine\DBAL\Connection;
use Doctrine\DBAL\DriverManager;
use Nativella\Runtime;

/**
 * The SQLite database of the application.
 */
final class Database
{
    public const string FILE_NAME = 'snippets.sqlite';

    private function __construct()
    {
    }

    /**
     * In the data directory Nativella gives the application, unique to the
     * application and the user; in var/ outside Nativella (php -S).
     */
    public static function path(string $projectDir): string
    {
        return (Runtime::dataDir() ?? $projectDir.'/var').'/'.self::FILE_NAME;
    }

    public static function connect(string $path): Connection
    {
        $directory = \dirname($path);
        if (!is_dir($directory) && !mkdir($directory, 0o777, true) && !is_dir($directory)) {
            throw new \RuntimeException(\sprintf('The directory "%s" of the database cannot be created.', $directory));
        }

        return DriverManager::getConnection(['driver' => 'pdo_sqlite', 'path' => $path]);
    }
}
