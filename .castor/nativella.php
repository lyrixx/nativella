<?php

namespace nativella;

use Castor\Attribute\AsArgument;
use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use project\Project;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Finder\Finder;
use Symfony\Component\Finder\SplFileInfo;

use function Castor\context;
use function Castor\finder;
use function Castor\fs;
use function Castor\io;
use function Castor\run;
use function Castor\variable;
use function Castor\with;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\docker_compose_run;
use function docker\docker_exit_code;
use function docker\run_in_builder;
use function project\demo;
use function project\demo_names;

/**
 * A desktop application needs the display of the host: it runs on the host by
 * default, with its cargo, its libphp and its WebKitGTK (the packages of
 * docs/development.md). --docker runs it in the builder instead, which
 * reaches the Wayland (or X11) and D-Bus sockets of the session, or in the
 * static service with --static.
 */
#[AsTask(name: 'run', description: 'Runs a demo as a desktop application (on the host, or in the builder with --docker)')]
function run_demo(
    #[AsArgument(description: 'The demo to run, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'Run it in the builder, on the display of the host')]
    bool $docker = false,
    #[AsOption(description: 'Link the static PHP of "castor static-php:build", as on macOS')]
    bool $static = false,
): int {
    $project = installed_demo($demo);

    io()->title("Running {$project->path}");

    $command = ['cargo', 'run', '-p', 'nativella', '--', 'run', $project->path];

    if (!$docker) {
        // As in the host context: the paths of the static PHP are those of the host
        return with(
            static fn (): int => (int) run($command, context: context()->withEnvironment($static ? \static_php\environment() : \static_php\system_php_environment()))->getExitCode(),
            workingDirectory: variable('root_dir'),
            allowFailure: true,
            context: context()->withData(['run_on_host' => true]),
        );
    }

    if (!getenv('XDG_RUNTIME_DIR') || (!getenv('WAYLAND_DISPLAY') && !getenv('DISPLAY'))) {
        throw new \RuntimeException('--docker needs a desktop session: XDG_RUNTIME_DIR, and WAYLAND_DISPLAY or DISPLAY.');
    }

    $c = context()->withData(
        [
            'docker_compose_files' => [...context()['docker_compose_files'], 'docker-compose.desktop.yml'],
            'docker_compose_run_environment' => $static ? \static_php\environment() : \static_php\system_php_environment(),
        ],
        recursive: false,
    );

    return docker_exit_code($command, $c, $static ? \static_php\SERVICE : null, workDir: builder_path());
}

/**
 * The executables link the static PHP of "castor static-php:build", as the
 * release does, in the static service (Ubuntu 22.04, for its old glibc): they
 * need no PHP on the machine. With --no-static, they link the libphp and the
 * WebKitGTK of the builder (Debian), dynamically, and need them. Each demo
 * ships staged for production: without its dev dependencies, in the
 * production environment.
 *
 * @param list<string> $demos
 */
#[AsTask(name: 'build', description: 'Builds the nativella command and the standalone executables of the demos into dist/')]
function build_demos(
    #[AsArgument(description: 'The demos to build (default: all)', autocomplete: 'project\complete_demo_names')]
    array $demos = [],
    #[AsOption(description: 'Link the static PHP of "castor static-php:build", so that the executables need no PHP on the machine, or the PHP of the system', mode: InputOption::VALUE_NEGATABLE)]
    bool $static = true,
): void {
    // Staging installs their dependencies: they need not be installed here
    $projects = array_map(demo(...), $demos ?: demo_names());
    $environment = $static ? \static_php\environment() : \static_php\system_php_environment();
    $service = $static ? \static_php\SERVICE : null;

    io()->title('Building the nativella command');

    run_in_builder(['cargo', 'build', '-p', 'nativella', '--release'], environment: $environment, service: $service);
    fs()->copy(target_dir($environment).'/release/nativella', variable('root_dir').'/dist/nativella', overwriteNewerFiles: true);

    foreach ($projects as $project) {
        io()->section("Building {$project->path} into dist/{$project->name()}");

        // Staged in the builder, which has PHP and Composer
        $stage = stage($project);
        run_in_builder(['cargo', 'run', '-q', '-p', 'nativella', '--release', '--', 'build', $stage, '-o', "dist/{$project->name()}"], environment: $environment, service: $service);
    }

    io()->success('The executables are in dist/.');
}

/**
 * What a GitHub release publishes: nativella-<platform> and
 * nativella-demo-<demo>-<platform>, with SHA256SUMS-<platform>. On Linux,
 * only executables that run on the glibc and the libstdc++ of the static
 * service, built against the static PHP.
 */
#[AsTask(description: 'Copies the executables of dist/ into release/, named after the platform, with their checksums')]
function package(): void
{
    $root = variable('root_dir');
    $platform = builder_platform();

    fs()->remove("{$root}/release");

    $checksums = '';
    foreach (finder()->in("{$root}/dist")->files()->depth(0)->sortByName() as $file) {
        if (str_starts_with($platform, 'linux-')) {
            check_symbol_versions("dist/{$file->getFilename()}");
        }

        $name = 'nativella' === $file->getFilename() ? "nativella-{$platform}" : "nativella-demo-{$file->getFilename()}-{$platform}";
        fs()->copy($file->getPathname(), "{$root}/release/{$name}");
        $checksums .= hash_file('sha256', $file->getPathname())."  {$name}\n";
    }

    fs()->dumpFile("{$root}/release/SHA256SUMS-{$platform}", $checksums);

    io()->success("The release files of {$platform} are in release/.");
}

/**
 * Fails if the executable needs a glibc, or a libstdc++, newer than the one of
 * the static service: the distributions that have an older one could not run
 * it.
 */
function check_symbol_versions(string $executable): void
{
    $symbols = docker_compose_run(['objdump', '-T', $executable], context()->withQuiet(), \static_php\SERVICE, workDir: builder_path());
    if (!$symbols->isSuccessful()) {
        throw new \RuntimeException(\sprintf('Cannot read the symbols of %s (%s).', $executable, trim($symbols->getErrorOutput())));
    }

    $glibc = newest_symbol_version($symbols->getOutput(), 'GLIBC');
    if (null === $glibc) {
        throw new \RuntimeException(\sprintf('%s does not link the glibc: is it an executable for Linux?', $executable));
    }

    if (version_compare($glibc, \static_php\GLIBC_VERSION, '>')) {
        throw new \RuntimeException(\sprintf('%s needs glibc %s, newer than the %s of the static service: build it against the static PHP, with "castor nativella:build", without --no-static.', $executable, $glibc, \static_php\GLIBC_VERSION));
    }

    // For intl and ICU, which are C++: without them, no libstdc++
    $glibcxx = newest_symbol_version($symbols->getOutput(), 'GLIBCXX');
    if (null !== $glibcxx && version_compare($glibcxx, \static_php\GLIBCXX_VERSION, '>')) {
        throw new \RuntimeException(\sprintf('%s needs the GLIBCXX_%s of libstdc++, newer than the GLIBCXX_%s of the static service: build it in the static service, with "castor nativella:build".', $executable, $glibcxx, \static_php\GLIBCXX_VERSION));
    }

    io()->writeln(\sprintf('<info>✔</info> %s runs on glibc %s%s.', $executable, $glibc, null === $glibcxx ? '' : ", with the GLIBCXX_{$glibcxx} of libstdc++"));
}

/**
 * The newest version of a library that symbols ask for, in the output of
 * `objdump -T`: 2.35 for "GLIBC" when the newest is GLIBC_2.35.
 */
function newest_symbol_version(string $symbols, string $library): ?string
{
    preg_match_all('/\b'.$library.'_(\d+(?:\.\d+)+)\b/', $symbols, $matches);
    $versions = $matches[1];
    usort($versions, static fn (string $a, string $b): int => version_compare($a, $b));

    return array_pop($versions);
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
    #[AsOption(description: 'Link the static PHP of "castor static-php:build", as on macOS')]
    bool $static = false,
): int {
    $projects = installed_demos($demos);
    $environment = $static ? \static_php\environment() : \static_php\system_php_environment();
    $service = $static ? \static_php\SERVICE : null;

    io()->title('Smoke testing the demos');

    // Once, with the output: the first build takes a while
    run_in_builder(['cargo', 'build', '-p', 'nativella-runtime', '--example', 'request'], environment: $environment, service: $service);

    $c = context()->withQuiet()->withData(['docker_compose_run_environment' => $environment], recursive: false);

    $exitCode = 0;
    foreach ($projects as $project) {
        io()->section("Smoke testing {$project->path}");

        $dataDir = ".home/.local/share/nativella-smoke/{$project->name()}";
        fs()->remove(variable('root_dir')."/{$dataDir}");
        fs()->mkdir(variable('root_dir')."/{$dataDir}");

        $request = ['cargo', 'run', '-q', '-p', 'nativella-runtime', '--example', 'request', '--', $project->path];
        $server = ['--server', 'NATIVELLA_DATA_DIR='.builder_path($dataDir)];

        $requests = [
            'The "app.booted" event' => [...$request, 'POST', '/_nativella/event', '--internal', '--data', '{"name":"app.booted","payload":{}}', ...$server],
            'The home page' => [...$request, 'GET', '/', ...$server],
        ];

        foreach ($requests as $label => $command) {
            $process = docker_compose_run($command, $c, $service, workDir: builder_path());
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
 * What mobile:ios compiles into the application: for the Mobile workflow,
 * whose macOS runners have no Docker to make it.
 */
#[AsTask(name: 'bundle', description: 'Bundles a demo for an iOS build, staged for production, into target/nativella/app.bundle')]
function bundle_demo(
    #[AsArgument(description: 'The demo to bundle, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
): void {
    io()->title("Bundling demos/{$demo}");

    io()->success('The bundle: '.bundle(demo($demo)));
}

/**
 * What the CI caches of a Rust build: the dependencies, built once for a
 * Cargo.lock. Not the crates of the workspace: a checkout makes their sources
 * newer than their build, which cargo then builds again anyway. Removes them
 * from every profile directory of target/ (target/debug/,
 * target/docker/aarch64-linux-android/debug/...).
 */
#[AsTask(name: 'prune-target', description: 'Removes the build of the crates of the workspace from target/, but not of their dependencies (what the CI caches)')]
function prune_target(): void
{
    $root = variable('root_dir');

    // Each one has its .fingerprint/
    $profiles = !is_dir("{$root}/target") ? [] : array_map(
        static fn (SplFileInfo $fingerprints): string => $fingerprints->getPath(),
        iterator_to_array(finder()->in("{$root}/target")->directories()->ignoreDotFiles(false)->depth('< 4')->name('.fingerprint'), false),
    );

    if (!$profiles) {
        io()->comment('Nothing built yet.');

        return;
    }

    $metadata = docker_compose_run(['cargo', 'metadata', '--no-deps', '--format-version', '1'], context()->withQuiet(), workDir: builder_path());
    if (!$metadata->isSuccessful()) {
        throw new \RuntimeException(\sprintf('Cannot list the crates of the workspace (%s).', trim($metadata->getErrorOutput())));
    }

    /** @var list<array{name: string, targets: list<array{name: string}>}> $packages */
    $packages = json_decode($metadata->getOutput(), true, flags: \JSON_THROW_ON_ERROR)['packages'];
    $alternatives = static fn (array $names): string => implode('|', array_map(static fn (string $name): string => preg_quote($name, '/'), $names));
    // The files of a target (lib, binary, example, test): <name>,
    // <name>-<hash>, lib<name>.rlib..., its dashes as underscores. The
    // directories of a package: <name>-<hash>, its build script and its
    // fingerprints.
    $targetFiles = '/^(lib)?('.$alternatives(array_map(static fn (array $target): string => str_replace('-', '_', $target['name']), array_merge(...array_column($packages, 'targets')))).')([-.]|$)/';
    $packageDirectories = '/^('.$alternatives(array_column($packages, 'name')).')-[0-9a-f]+$/';

    fs()->remove(finder()->in($profiles)->ignoreDotFiles(false)->depth('< 2')->filter(static fn (SplFileInfo $file): bool => match ($file->getRelativePath()) {
        '', 'deps', 'examples', 'incremental' => 1 === preg_match($targetFiles, $file->getFilename()),
        '.fingerprint', 'build' => 1 === preg_match($packageDirectories, $file->getFilename()),
        default => false,
    }));

    io()->success(\sprintf('Removed the crates of the workspace from %s.', implode(', ', array_map(static fn (string $profile): string => substr($profile, \strlen($root) + 1), $profiles))));
}

/**
 * Copies a project to target/nativella/stage/ and prepares it for production
 * there: the dependencies without the dev ones, copied rather than linked
 * from the path repositories of the demos, and the production environment.
 *
 * @return string the staged directory, for the commands of the builder
 */
function stage(Project $project): string
{
    $root = variable('root_dir');
    $stage = "target/nativella/stage/{$project->path}";

    // The packages, for the path repositories of the demos
    mirror(
        finder()->in("{$root}/packages")->ignoreDotFiles(false)->exclude('vendor')->notName('composer.lock'),
        "{$root}/packages",
        "{$root}/target/nativella/stage/packages",
    );
    // Not bootstrap/cache/*.php either: Laravel's package discovery of the
    // dev install, which lists dev packages
    mirror(
        finder()->in($project->directory())->ignoreDotFiles(false)
            ->exclude(['node_modules', 'tests', 'var', 'vendor'])
            ->notPath(['#^bootstrap/cache/[^/]+\.php$#', '#^storage/logs/.#'])
            ->notName(['.env.local', '.env.*.local']),
        $project->directory(),
        "{$root}/{$stage}",
    );

    $builderStage = builder_path($stage);
    run_in_builder(['composer', 'install', '--no-dev', '--optimize-autoloader', '--no-interaction', '--no-progress', '--no-scripts', '--quiet'], $builderStage, ['COMPOSER_MIRROR_PATH_REPOS' => '1']);

    if ($project->isSymfony()) {
        // The compiled .env of production. The cache is built on the
        // machine of the user: it holds absolute paths.
        run_in_builder(['composer', 'dump-env', 'prod', '--quiet'], $builderStage);
    } elseif ($project->isLaravel()) {
        // A fresh clone has no .env yet
        if (!is_file("{$root}/{$stage}/.env")) {
            fs()->copy("{$root}/{$stage}/.env.example", "{$root}/{$stage}/.env");
        }

        run_in_builder(['php', 'artisan', 'package:discover', '--quiet'], $builderStage);

        $env = (string) file_get_contents("{$root}/{$stage}/.env");
        fs()->dumpFile("{$root}/{$stage}/.env", (string) preg_replace(['/^APP_ENV=.*$/m', '/^APP_DEBUG=.*$/m'], ['APP_ENV=production', 'APP_DEBUG=false'], $env));
    }

    return $builderStage;
}

/**
 * Bundles a project for an iOS build (`nativella bundle`), staged for
 * production, into target/nativella/app.bundle: the build compiles it into
 * the application.
 *
 * @return string the bundle, from the root of the repository
 */
function bundle(Project $project): string
{
    $bundle = 'target/nativella/app.bundle';

    run_in_builder(['cargo', 'run', '-q', '-p', 'nativella', '--', 'bundle', stage($project), '-o', builder_path($bundle)]);

    return $bundle;
}

/**
 * Where cargo builds, on the host, for the environment of a cargo command of
 * the builder.
 *
 * @param array<string, string> $environment
 */
function target_dir(array $environment): string
{
    $root = variable('root_dir');

    if (isset($environment['CARGO_TARGET_DIR'])) {
        return $root.substr($environment['CARGO_TARGET_DIR'], \strlen(builder_path()));
    }

    // The builder sets CARGO_TARGET_DIR to target/docker
    return context()['run_on_host'] ? "{$root}/target" : "{$root}/target/docker";
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
 * Makes $to a copy of the files $files finds in $from, and of them only.
 */
function mirror(Finder $files, string $from, string $to): void
{
    fs()->remove($to);
    fs()->mirror($from, $to, $files);
}
