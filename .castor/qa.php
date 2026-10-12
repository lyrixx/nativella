<?php

namespace qa;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use project\Project;
use Symfony\Component\Console\Input\InputOption;

use function Castor\context;
use function Castor\io;
use function Castor\variable;
use function docker\docker_exit_code;
use function project\select;

const TOOLS = ['php-cs-fixer', 'phpstan', 'twig-cs-fixer'];

#[AsTask(description: 'Runs all QA tasks', aliases: ['qa'])]
function all(
    #[AsOption(description: 'Only report coding standard violations, do not fix them')]
    bool $dryRun = false,
): int {
    $cs = cs($dryRun);
    $twigCs = twigCs($dryRun);
    $phpstan = phpstan();
    $phpunit = phpunit();
    $rust = rust();

    return max($cs, $twigCs, $phpstan, $phpunit, $rust);
}

#[AsTask(description: 'Installs tooling')]
function install(): void
{
    io()->title('Installing tooling');

    foreach (TOOLS as $tool) {
        if (0 !== docker_exit_code(['composer', 'install', '-o'], workDir: "/var/www/tools/{$tool}")) {
            throw new \RuntimeException(\sprintf('"composer install" failed in tools/%s.', $tool));
        }
    }

    install_mkdocs();
}

/**
 * MkDocs, which builds the documentation, in a virtual environment of its
 * own: tools/mkdocs/.venv, with the versions of tools/mkdocs/requirements.txt.
 * Cheap when installed already.
 */
function install_mkdocs(): void
{
    $commands = [
        ['python3', '-m', 'venv', '.venv'],
        ['.venv/bin/pip', 'install', '--quiet', '--disable-pip-version-check', '-r', 'requirements.txt'],
    ];

    foreach ($commands as $command) {
        if (0 !== docker_exit_code($command, workDir: '/var/www/tools/mkdocs')) {
            throw new \RuntimeException(\sprintf('"%s" failed in tools/mkdocs.', implode(' ', $command)));
        }
    }
}

#[AsTask(description: 'Updates tooling')]
function update(): void
{
    io()->title('Updating QA tooling');

    foreach (TOOLS as $tool) {
        if (0 !== docker_exit_code(['composer', 'update', '-o'], workDir: "/var/www/tools/{$tool}")) {
            throw new \RuntimeException(\sprintf('"composer update" failed in tools/%s.', $tool));
        }
    }
}

/**
 * The test suite of each package and demo: `php artisan test` for a Laravel
 * application, PHPUnit otherwise.
 *
 * @param list<string> $project
 */
#[AsTask(description: 'Runs the test suites of the projects (PHPUnit)', aliases: ['phpunit'])]
function phpunit(
    #[AsOption(description: 'The project to test, e.g. demos/symfony (repeatable, default: all)', mode: InputOption::VALUE_IS_ARRAY | InputOption::VALUE_REQUIRED, autocomplete: 'project\complete_project_paths')]
    array $project = [],
): int {
    $exitCode = 0;

    foreach (installed_projects(select($project)) as $p) {
        if (!$p->hasTests()) {
            continue;
        }

        io()->section("Running the tests of {$p->path}...");

        $command = $p->isLaravel() ? ['php', 'artisan', 'test'] : ['vendor/bin/phpunit'];

        $exitCode = max($exitCode, docker_exit_code($command, workDir: $p->containerDirectory()));
    }

    return $exitCode;
}

/**
 * The Castor tasks with phpstan.neon, then each project from its own
 * directory: PHPStan finds its classes through its composer.json and vendor/.
 * A project with a configuration of its own (phpstan.dist.neon...) gets it,
 * the others get tools/phpstan/project.neon over their PSR-4 directories.
 *
 * @param list<string> $project
 */
#[AsTask(description: 'Runs PHPStan', aliases: ['phpstan'])]
function phpstan(
    #[AsOption(description: 'The project to analyse, e.g. demos/symfony (repeatable, default: all, and the Castor tasks)', mode: InputOption::VALUE_IS_ARRAY | InputOption::VALUE_REQUIRED, autocomplete: 'project\complete_project_paths')]
    array $project = [],
): int {
    install_if_missing('phpstan');

    $command = ['phpstan', 'analyse', '--memory-limit=-1', '-v'];
    $exitCode = 0;

    if (!$project) {
        io()->section('Running PHPStan on the Castor tasks...');

        $exitCode = docker_exit_code($command, workDir: '/var/www');
    }

    foreach (installed_projects(select($project)) as $p) {
        io()->section("Running PHPStan on {$p->path}...");

        $projectCommand = $command;
        if (!$p->hasPhpstanConfiguration()) {
            $directories = $p->sourceDirectories();
            if (!$directories) {
                io()->comment('No PSR-4 directory to analyse.');

                continue;
            }

            $projectCommand = [...$command, '-c', '/var/www/tools/phpstan/project.neon', ...$directories];
        }

        // One cache per project: they share no class
        $c = context()->withData(['docker_compose_run_environment' => [
            'PHPSTAN_TMP_DIR' => '/var/www/tools/phpstan/var/'.str_replace('/', '-', $p->path),
        ]]);

        $exitCode = max($exitCode, docker_exit_code($projectCommand, $c, workDir: $p->containerDirectory()));
    }

    return $exitCode;
}

#[AsTask(description: 'Runs Security audit')]
function securityAudit(): int
{
    $exitCode = 0;

    foreach (installed_projects(select([])) as $p) {
        io()->section("Auditing the dependencies of {$p->path}...");

        $exitCode = max($exitCode, docker_exit_code(['composer', 'audit'], workDir: $p->containerDirectory()));
    }

    return $exitCode;
}

#[AsTask(description: 'Fixes Coding Style', aliases: ['cs'])]
function cs(bool $dryRun = false): int
{
    install_if_missing('php-cs-fixer');

    io()->section('Running PHP CS Fixer...');

    if ($dryRun) {
        return docker_exit_code(['php-cs-fixer', 'fix', '--dry-run', '--diff'], workDir: '/var/www');
    }

    return docker_exit_code(['php-cs-fixer', 'fix', '-v'], workDir: '/var/www');
}

#[AsTask(description: 'Fixes Twig Coding Style', aliases: ['twig-cs'])]
function twigCs(bool $dryRun = false): int
{
    install_if_missing('twig-cs-fixer');

    io()->section('Running Twig CS Fixer...');

    if ($dryRun) {
        return docker_exit_code(['twig-cs-fixer'], workDir: '/var/www');
    }

    return docker_exit_code(['twig-cs-fixer', '--fix'], workDir: '/var/www');
}

/**
 * The crates of the workspace, built in the builder: its own target
 * directory, its own libphp.
 */
#[AsTask(description: 'Checks the Rust crates: format, Clippy and tests')]
function rust(): int
{
    io()->section('Checking the format of the Rust code...');
    $fmt = docker_exit_code(['cargo', 'fmt', '--all', '--check'], workDir: '/var/www');

    io()->section('Running Clippy...');
    $clippy = docker_exit_code(['cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'], workDir: '/var/www');

    io()->section('Running the Rust tests...');
    $test = docker_exit_code(['cargo', 'test', '--workspace'], workDir: '/var/www');

    return max($fmt, $clippy, $test);
}

function install_if_missing(string $tool): void
{
    if (!is_dir(variable('root_dir')."/tools/{$tool}/vendor")) {
        install();
    }
}

/**
 * @param list<Project> $projects
 *
 * @return list<Project>
 */
function installed_projects(array $projects): array
{
    $installed = [];
    foreach ($projects as $project) {
        if (!$project->isInstalled()) {
            io()->warning(\sprintf('%s is not installed, run "castor install" first.', $project->path));

            continue;
        }

        $installed[] = $project;
    }

    return $installed;
}
