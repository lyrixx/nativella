<?php

namespace nativella;

use Castor\Attribute\AsArgument;
use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use project\Project;

use function Castor\context;
use function Castor\fs;
use function Castor\io;
use function Castor\run;
use function Castor\variable;
use function docker\docker_compose_run;
use function docker\docker_exit_code;
use function project\demo;
use function project\demo_names;

/**
 * A desktop application needs the display of the host: it runs on the host by
 * default, with its cargo, its libphp and its WebKitGTK (the packages of
 * docs/installation.md). --docker runs it in the builder instead, which
 * reaches the Wayland (or X11) and D-Bus sockets of the session.
 */
#[AsTask(name: 'run', description: 'Runs a demo as a desktop application (on the host, or in the builder with --docker)')]
function run_demo(
    #[AsArgument(description: 'The demo to run, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'Run it in the builder, on the display of the host')]
    bool $docker = false,
): int {
    $project = installed_demo($demo);

    io()->title("Running {$project->path}");

    $command = ['cargo', 'run', '-p', 'nativella', '--', 'run', $project->path];

    if (!$docker) {
        return (int) run($command, context: context()->withAllowFailure()->withWorkingDirectory(variable('root_dir')))->getExitCode();
    }

    if (!getenv('XDG_RUNTIME_DIR') || (!getenv('WAYLAND_DISPLAY') && !getenv('DISPLAY'))) {
        throw new \RuntimeException('--docker needs a desktop session: XDG_RUNTIME_DIR, and WAYLAND_DISPLAY or DISPLAY.');
    }

    $c = context()->withData(
        ['docker_compose_files' => [...context()['docker_compose_files'], 'docker-compose.desktop.yml']],
        recursive: false,
    );

    return docker_exit_code($command, $c, workDir: '/var/www');
}

/**
 * In the builder, as the release does: the executables link the libphp and
 * the WebKitGTK of the image (Debian), dynamically. Each demo ships staged
 * for production (mobile/stage.sh): without its dev dependencies, in the
 * production environment.
 *
 * @param list<string> $demos
 */
#[AsTask(name: 'build', description: 'Builds the nativella command and the standalone executables of the demos into dist/')]
function build_demos(
    #[AsArgument(description: 'The demos to build (default: all)', autocomplete: 'project\complete_demo_names')]
    array $demos = [],
): void {
    // Staging installs their dependencies: they need not be installed here
    $projects = array_map(demo(...), $demos ?: demo_names());

    io()->title('Building the nativella command');

    cargo(['build', '-p', 'nativella', '--release']);
    run_in_builder(['bash', '-c', 'mkdir -p dist && cp "$CARGO_TARGET_DIR/release/nativella" dist/nativella']);

    foreach ($projects as $project) {
        io()->section("Building {$project->path} into dist/{$project->name()}");

        cargo(['run', '-q', '-p', 'nativella', '--release', '--', 'build', stage($project), '-o', "dist/{$project->name()}"]);
    }

    io()->success('The executables are in dist/.');
}

/**
 * Serves requests to each demo through the runtime, without any window: the
 * "app.booted" event (which migrates the database), then the home page. Each
 * demo gets an empty data directory, in .home/.
 *
 * @param list<string> $demos
 */
#[AsTask(description: 'Checks that the demos answer through the runtime, without any window')]
function smoke(
    #[AsArgument(description: 'The demos to check (default: all)', autocomplete: 'project\complete_demo_names')]
    array $demos = [],
): int {
    $projects = installed_demos($demos);

    io()->title('Smoke testing the demos');

    // Once, with the output: the first build takes a while
    cargo(['build', '-p', 'nativella-runtime', '--example', 'request']);

    $exitCode = 0;
    foreach ($projects as $project) {
        io()->section("Smoke testing {$project->path}");

        $dataDir = ".local/share/nativella-smoke/{$project->name()}";
        fs()->remove(variable('root_dir')."/.home/{$dataDir}");
        fs()->mkdir(variable('root_dir')."/.home/{$dataDir}");

        $request = ['cargo', 'run', '-q', '-p', 'nativella-runtime', '--example', 'request', '--', $project->path];
        $server = ['--server', "NATIVELLA_DATA_DIR=/home/app/{$dataDir}"];

        $requests = [
            'The "app.booted" event' => [...$request, 'POST', '/_nativella/event', '--internal', '--data', '{"name":"app.booted","payload":{}}', ...$server],
            'The home page' => [...$request, 'GET', '/', ...$server],
        ];

        foreach ($requests as $label => $command) {
            $process = docker_compose_run($command, context()->withQuiet(), workDir: '/var/www');
            $statusLine = strtok($process->getOutput(), "\n") ?: '';

            if (0 === $process->getExitCode() && preg_match('{^HTTP/\S+ [23]\d\d\b}', $statusLine)) {
                io()->writeln("<info>✔</info> {$label}: {$statusLine}");

                continue;
            }

            io()->writeln("<error>✘</error> {$label}: ".($statusLine ?: 'no response'));
            io()->writeln($process->getOutput().$process->getErrorOutput());
            $exitCode = 1;
        }
    }

    return $exitCode;
}

/**
 * Copies a project to target/nativella/stage/ and prepares it for production
 * there, with mobile/stage.sh.
 *
 * @return string the staged directory, in the builder
 */
function stage(Project $project): string
{
    $process = docker_compose_run(['mobile/stage.sh', $project->path], workDir: '/var/www');
    $lines = explode("\n", trim($process->getOutput()));

    if (0 !== $process->getExitCode() || !str_starts_with(end($lines), '/var/www/')) {
        throw new \RuntimeException(\sprintf('%s cannot be staged.', $project->path));
    }

    return end($lines);
}

function installed_demo(string $name): Project
{
    $demo = demo($name);

    if (!$demo->isInstalled()) {
        throw new \RuntimeException(\sprintf('%s is not installed: run "castor install" first.', $demo->path));
    }

    return $demo;
}

/**
 * @param list<string> $names every demo when empty
 *
 * @return list<Project>
 */
function installed_demos(array $names): array
{
    $names = $names ?: demo_names();

    return array_map(installed_demo(...), $names);
}

/**
 * @param list<string> $arguments
 */
function cargo(array $arguments): void
{
    run_in_builder(['cargo', ...$arguments]);
}

/**
 * Runs a command in the builder, from the root of the repository.
 *
 * @param list<string> $command
 */
function run_in_builder(array $command): void
{
    if (0 !== docker_exit_code($command, workDir: '/var/www')) {
        throw new \RuntimeException(\sprintf('"%s" failed.', implode(' ', $command)));
    }
}
