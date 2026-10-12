<?php

use Symfony\Component\Dotenv\Dotenv;

require dirname(__DIR__).'/vendor/autoload.php';

new Dotenv()->bootEnv(dirname(__DIR__).'/.env');

if ($_SERVER['APP_DEBUG']) {
    umask(0o000);
}

// A fresh database for every run, migrated as the application does at boot.
@unlink(dirname(__DIR__).'/var/test.db');
passthru(sprintf('php %s/bin/console doctrine:migrations:migrate --env=test --no-interaction --quiet', escapeshellarg(dirname(__DIR__))), $status);
if (0 !== $status) {
    throw new RuntimeException('The test database could not be migrated.');
}
