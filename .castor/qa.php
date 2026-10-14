<?php

namespace qa;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use project\Project;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Contracts\HttpClient\Exception\ExceptionInterface as HttpExceptionInterface;

use function Castor\context;
use function Castor\http_request;
use function Castor\io;
use function Castor\variable;
use function docker\builder_path;
use function docker\docker_exit_code;
use function project\demo_names;
use function project\select;
use function worktree\get_worktree_name;
use function worktree\get_worktree_ports;

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
        if (0 !== docker_exit_code(['composer', 'install', '-o'], workDir: builder_path("tools/{$tool}"))) {
            throw new \RuntimeException(\sprintf('"composer install" failed in tools/%s.', $tool));
        }
    }

    install_zensical();
}

/**
 * Zensical, which builds the documentation, in a virtual environment of its
 * own: tools/zensical/.venv, with the versions of
 * tools/zensical/requirements.txt. Cheap when installed already.
 */
function install_zensical(): void
{
    $commands = [
        ['python3', '-m', 'venv', '.venv'],
        ['.venv/bin/pip', 'install', '--quiet', '--disable-pip-version-check', '-r', 'requirements.txt'],
    ];

    foreach ($commands as $command) {
        if (0 !== docker_exit_code($command, workDir: builder_path('tools/zensical'))) {
            throw new \RuntimeException(\sprintf('"%s" failed in tools/zensical.', implode(' ', $command)));
        }
    }
}

#[AsTask(description: 'Updates tooling')]
function update(): void
{
    io()->title('Updating QA tooling');

    foreach (TOOLS as $tool) {
        if (0 !== docker_exit_code(['composer', 'update', '-o'], workDir: builder_path("tools/{$tool}"))) {
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

        $exitCode = max($exitCode, docker_exit_code($command, workDir: $p->builderDirectory()));
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

        $exitCode = docker_exit_code($command, workDir: builder_path());
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

            $projectCommand = [...$command, '-c', builder_path('tools/phpstan/project.neon'), ...$directories];
        }

        // One cache per project: they share no class
        $c = context()->withData(['docker_compose_run_environment' => [
            'PHPSTAN_TMP_DIR' => builder_path('tools/phpstan/var/'.str_replace('/', '-', $p->path)),
        ]]);

        $exitCode = max($exitCode, docker_exit_code($projectCommand, $c, workDir: $p->builderDirectory()));
    }

    return $exitCode;
}

#[AsTask(description: 'Runs Security audit')]
function securityAudit(): int
{
    $exitCode = 0;

    foreach (installed_projects(select([])) as $p) {
        io()->section("Auditing the dependencies of {$p->path}...");

        $exitCode = max($exitCode, docker_exit_code(['composer', 'audit'], workDir: $p->builderDirectory()));
    }

    return $exitCode;
}

#[AsTask(description: 'Fixes Coding Style', aliases: ['cs'])]
function cs(bool $dryRun = false): int
{
    install_if_missing('php-cs-fixer');

    io()->section('Running PHP CS Fixer...');

    if ($dryRun) {
        return docker_exit_code(['php-cs-fixer', 'fix', '--dry-run', '--diff'], workDir: builder_path());
    }

    return docker_exit_code(['php-cs-fixer', 'fix', '-v'], workDir: builder_path());
}

#[AsTask(description: 'Fixes Twig Coding Style', aliases: ['twig-cs'])]
function twigCs(bool $dryRun = false): int
{
    install_if_missing('twig-cs-fixer');

    io()->section('Running Twig CS Fixer...');

    if ($dryRun) {
        return docker_exit_code(['twig-cs-fixer'], workDir: builder_path());
    }

    return docker_exit_code(['twig-cs-fixer', '--fix'], workDir: builder_path());
}

/**
 * The crates of the workspace, built in the builder: its own target
 * directory, its own libphp; or the static PHP of "castor static-php:build"
 * with --static, as on macOS, in the static service.
 */
#[AsTask(description: 'Checks the Rust crates: format, Clippy and tests')]
function rust(
    #[AsOption(description: 'Link the static PHP of "castor static-php:build"')]
    bool $static = false,
): int {
    $c = context()->withData(['docker_compose_run_environment' => $static ? \static_php\environment() : \static_php\system_php_environment()], recursive: false);
    $service = $static ? \static_php\SERVICE : null;

    io()->section('Checking the format of the Rust code...');
    $fmt = docker_exit_code(['cargo', 'fmt', '--all', '--check'], $c, $service, workDir: builder_path());

    io()->section('Running Clippy...');
    $clippy = docker_exit_code(['cargo', 'clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'], $c, $service, workDir: builder_path());

    io()->section('Running the Rust tests...');
    $test = docker_exit_code(['cargo', 'test', '--workspace'], $c, $service, workDir: builder_path());

    return max($fmt, $clippy, $test);
}

/**
 * Through the router, as a browser: the website on the root domain and www,
 * each demo on its own subdomain, without the native features. The requests
 * go to 127.0.0.1: the domains need not resolve.
 */
#[AsTask(description: 'Checks that the frontend serves the website and the demos (castor start first)')]
function frontend(): int
{
    $rootDomain = variable('root_domain');
    $port = get_worktree_ports(get_worktree_name())['https'];

    // The text a page must hold, if any
    $pages = [$rootDomain => 'Nativella', "www.{$rootDomain}" => 'Nativella'];
    foreach (demo_names() as $demo) {
        $pages["{$demo}.{$rootDomain}"] = null;
    }

    $exitCode = 0;
    foreach ($pages as $host => $text) {
        $url = 443 === $port ? "https://{$host}/" : "https://{$host}:{$port}/";

        try {
            $content = http_request('GET', $url, [
                'resolve' => [$host => '127.0.0.1'],
                // Self-signed, or signed by the local CA of mkcert
                'verify_host' => false,
                'verify_peer' => false,
            ])->getContent();
        } catch (HttpExceptionInterface $e) {
            io()->writeln("<error>✘</error> {$url}: {$e->getMessage()}");
            $exitCode = 1;

            continue;
        }

        if (null !== $text && !str_contains($content, $text)) {
            io()->writeln("<error>✘</error> {$url}: the page does not say \"{$text}\".");
            $exitCode = 1;

            continue;
        }

        io()->writeln("<info>✔</info> {$url}");
    }

    return $exitCode;
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
