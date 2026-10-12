<?php

namespace project;

use Symfony\Component\Console\Completion\CompletionInput;

use function docker\docker_exit_code;

/**
 * The context's root_dir, without the context: the default variables need the
 * demos to build the domains.
 */
function root_dir(): string
{
    return \dirname(__DIR__);
}

/**
 * Every directory with a composer.json: the packages first, as the demos and
 * the website may depend on them, then the demos, then the website. Found at
 * each call, so that a new project needs no change here.
 *
 * @return list<Project>
 */
function all(): array
{
    $rootDir = root_dir();

    $projects = [];
    foreach (['packages/*', 'demos/*', 'website'] as $pattern) {
        $files = glob("{$rootDir}/{$pattern}/composer.json") ?: [];
        sort($files);
        foreach ($files as $file) {
            $projects[] = new Project(substr(\dirname($file), \strlen($rootDir) + 1));
        }
    }

    return $projects;
}

/**
 * The demos that the desktop application and the frontend can serve.
 *
 * @return list<Project>
 */
function demos(): array
{
    return array_values(array_filter(
        all(),
        static fn (Project $project): bool => $project->isDemo() && $project->hasFile('public/index.php'),
    ));
}

function demo(string $name): Project
{
    foreach (demos() as $demo) {
        if ($demo->name() === $name) {
            return $demo;
        }
    }

    throw new \RuntimeException(\sprintf('There is no demo "%s": pick one of %s.', $name, implode(', ', demo_names())));
}

/**
 * @return list<string>
 */
function demo_names(): array
{
    return array_map(static fn (Project $demo): string => $demo->name(), demos());
}

/**
 * @param list<string> $paths the paths of projects ("demos/symfony"), every project when empty
 *
 * @return list<Project>
 */
function select(array $paths): array
{
    if (!$paths) {
        return all();
    }

    $projects = [];
    foreach ($paths as $path) {
        $path = trim($path, '/');
        $matches = array_filter(all(), static fn (Project $project): bool => $project->path === $path);
        if (!$matches) {
            throw new \RuntimeException(\sprintf('There is no project "%s": pick one of %s.', $path, implode(', ', project_paths())));
        }
        $projects[] = reset($matches);
    }

    return $projects;
}

/**
 * @return list<string>
 */
function project_paths(): array
{
    return array_map(static fn (Project $project): string => $project->path, all());
}

/**
 * Autocompletion of the demo arguments.
 *
 * @return list<string>
 */
function complete_demo_names(CompletionInput $input): array
{
    return demo_names();
}

/**
 * Autocompletion of the project options.
 *
 * @return list<string>
 */
function complete_project_paths(CompletionInput $input): array
{
    return project_paths();
}

/**
 * Runs a command in the builder, from the directory of the project.
 *
 * @param list<string> $command
 */
function run_or_fail(Project $project, array $command): void
{
    if (0 !== docker_exit_code($command, workDir: $project->builderDirectory())) {
        throw new \RuntimeException(\sprintf('"%s" failed in %s.', implode(' ', $command), $project->path));
    }
}
