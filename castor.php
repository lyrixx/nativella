<?php

use Castor\Attribute\AsTask;

use function Castor\guard_min_version;
use function Castor\import;
use function Castor\io;
use function Castor\notify;
use function docker\about;
use function docker\build;
use function docker\up;
use function project\run_or_fail;

defined('CASTOR_USE_CHDIR') || define('CASTOR_USE_CHDIR', true);

guard_min_version('1.8.0');

import(__DIR__.'/.castor');

/**
 * @return array{project_name: string, root_domain: string, extra_domains: string[], php_version: string, registry: ?string}
 */
function create_default_variables(): array
{
    $projectName = 'nativella';
    $tld = 'test';
    $rootDomain = "{$projectName}.{$tld}";

    return [
        'project_name' => $projectName,
        'root_domain' => $rootDomain,
        // The root domain and www serve the website, <demo>.<root domain> serves
        // demos/<demo> (see infrastructure/docker/services/php/nginx/conf.d/default.conf)
        'extra_domains' => [
            "www.{$rootDomain}",
            ...array_map(static fn (string $demo): string => "{$demo}.{$rootDomain}", project\demo_names()),
        ],
        // The PHP that Nativella embeds, in the builder (libphp) as in the frontend
        'php_version' => '8.5',
        'registry' => $_SERVER['REGISTRY'] ?? null,
    ];
}

#[AsTask(description: 'Builds and starts the infrastructure, then installs the projects, builds the website and migrates the demos')]
function start(): void
{
    io()->title('Starting the stack');

    build();
    install();
    website\build();
    migrate();
    up(profiles: ['default']);

    notify('The stack is now up and running.');
    io()->success('The stack is now up and running.');

    about();
}

#[AsTask(description: 'Installs every project (composer) and the QA tools', namespace: 'app', aliases: ['install'])]
function install(): void
{
    io()->title('Installing the projects');

    foreach (project\all() as $project) {
        io()->section("Installing {$project->path}");

        run_or_fail($project, ['composer', 'install', '-n', '--prefer-dist', '--optimize-autoloader']);

        // A Laravel application reads its key from .env, which is not versioned
        if ($project->isLaravel() && !$project->hasFile('.env') && $project->hasFile('.env.example')) {
            run_or_fail($project, ['cp', '.env.example', '.env']);
            run_or_fail($project, ['php', 'artisan', 'key:generate', '-n']);
        }
    }

    qa\install();
}

#[AsTask(description: 'Updates the dependencies of every project', namespace: 'app', aliases: ['update'])]
function update(bool $withTools = false): void
{
    io()->title('Updating dependencies...');

    foreach (project\all() as $project) {
        io()->section("Updating {$project->path}");

        run_or_fail($project, ['composer', 'update', '-n', '--optimize-autoloader']);
    }

    if ($withTools) {
        qa\update();
    }
}

/**
 * In a browser, no "app.booted" event migrates the database of a demo as
 * Nativella does: the demos that need it are migrated here.
 */
#[AsTask(description: 'Migrates the database of the demos, for the frontend', namespace: 'app:db', aliases: ['migrate'])]
function migrate(): void
{
    io()->title('Migrating the databases of the demos');

    foreach (project\demos() as $demo) {
        if ($demo->isSymfony() && $demo->hasFile('migrations')) {
            io()->section("Migrating {$demo->path}");
            run_or_fail($demo, ['bin/console', 'doctrine:migrations:migrate', '-n', '--allow-no-migration', '--all-or-nothing']);
        } elseif ($demo->isLaravel()) {
            io()->section("Migrating {$demo->path}");
            run_or_fail($demo, ['php', 'artisan', 'migrate', '-n', '--force']);
        }
    }
}
