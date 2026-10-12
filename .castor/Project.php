<?php

namespace project;

/**
 * A Composer project of the monorepo: a package, a demo or the website.
 */
final readonly class Project
{
    public function __construct(
        /** Relative to the root of the repository, e.g. "demos/symfony" */
        public string $path,
    ) {
    }

    /** "symfony" for demos/symfony */
    public function name(): string
    {
        return basename($this->path);
    }

    public function directory(): string
    {
        return root_dir().'/'.$this->path;
    }

    /** Where the builder mounts it */
    public function containerDirectory(): string
    {
        return '/var/www/'.$this->path;
    }

    public function isDemo(): bool
    {
        return str_starts_with($this->path, 'demos/');
    }

    public function isLaravel(): bool
    {
        return is_file($this->directory().'/artisan');
    }

    public function isSymfony(): bool
    {
        return is_file($this->directory().'/bin/console');
    }

    public function isInstalled(): bool
    {
        return is_file($this->directory().'/vendor/autoload.php');
    }

    public function hasFile(string $path): bool
    {
        return file_exists($this->directory().'/'.$path);
    }

    public function hasTests(): bool
    {
        foreach (['phpunit.xml', 'phpunit.xml.dist', 'phpunit.dist.xml'] as $configuration) {
            if ($this->hasFile($configuration)) {
                return true;
            }
        }

        return false;
    }

    public function hasPhpstanConfiguration(): bool
    {
        foreach (['phpstan.neon', 'phpstan.neon.dist', 'phpstan.dist.neon'] as $configuration) {
            if ($this->hasFile($configuration)) {
                return true;
            }
        }

        return false;
    }

    /**
     * The directories of the PSR-4 autoloading (autoload and autoload-dev of
     * composer.json): the code of the project, whatever its layout.
     *
     * @return list<string> relative to the project
     */
    public function sourceDirectories(): array
    {
        $composer = json_decode((string) file_get_contents($this->directory().'/composer.json'), true, flags: \JSON_THROW_ON_ERROR);

        $directories = [];
        foreach (['autoload', 'autoload-dev'] as $section) {
            foreach ($composer[$section]['psr-4'] ?? [] as $paths) {
                foreach ((array) $paths as $path) {
                    $path = rtrim($path, '/');
                    if (is_dir($this->directory().'/'.$path)) {
                        $directories[] = $path;
                    }
                }
            }
        }

        return array_values(array_unique($directories));
    }
}
