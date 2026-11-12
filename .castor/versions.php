<?php

namespace versions;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;

use function Castor\finder;
use function Castor\http_request;
use function Castor\io;
use function Castor\variable;
use function notices\workspace_package;

/**
 * The tag of a release: vX.Y.Z, with an optional -alphaN, -betaN or -rcN.
 * Keep in sync with the origins of .gitsplit.yml, and with the tags of
 * .github/workflows/release.yml and gitsplit.yml.
 */
const TAG = '/^v(\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\d+)?)$/';

/**
 * The crates, the PHP packages and the nativella command share one version,
 * the one of the workspace in the root Cargo.toml: every file that repeats it
 * must agree, and so must the tag of a release, which is immutable once
 * published. The Composer the workflows install on the runners is the one of
 * the builder too, which Dependabot updates in the Dockerfile only.
 */
#[AsTask(namespace: 'qa', name: 'versions', description: 'Checks that the version of the workspace is the one of every file that repeats it, and of a release tag')]
function check(
    #[AsOption(description: 'The tag of a release, e.g. v0.6.0, which must be the version of the workspace')]
    ?string $tag = null,
): int {
    $version = workspace_package('version');
    io()->section("Checking the versions against {$version}, the one of the workspace...");

    $mismatches = [
        ...tag($tag, $version),
        ...crates($version),
        ...composer_projects($version),
        ...workflows_composer(),
    ];

    if ([] === $mismatches) {
        io()->writeln('<info>✔</info> Every version agrees.');

        return 0;
    }

    foreach ($mismatches as $mismatch) {
        io()->writeln("<error>✘</error> {$mismatch}");
    }

    return 1;
}

/**
 * What no Dependabot ecosystem follows, and a release ships: the PHP that
 * every platform builds (static_php\PHP_RELEASE), the Rust of the images
 * (RUST_VERSION in their Dockerfile), and the SQLite that PHP links
 * (static_php\SQLITE_RELEASE). The weekly run of the PHP workflow fails here
 * when php.net, the stable channel of Rust or sqlite.org has a newer one: a
 * security release of PHP, above all.
 */
#[AsTask(namespace: 'qa', name: 'outdated', description: 'Checks that the pinned PHP, Rust and SQLite are the latest releases of PHP 8.5, of Rust stable and of SQLite')]
function outdated(): int
{
    io()->section('Checking the pinned PHP, Rust and SQLite against their latest releases...');

    $branch = implode('.', \array_slice(explode('.', \static_php\PHP_RELEASE), 0, 2));
    $php = json_decode(fetch("https://www.php.net/releases/index.php?json&version={$branch}"), true, flags: \JSON_THROW_ON_ERROR)['version']
        ?? throw new \RuntimeException(\sprintf('php.net names no release of PHP %s.', $branch));

    $dockerfile = 'infrastructure/docker/services/php/Dockerfile';
    preg_match_all('{^ARG RUST_VERSION=(\S+)$}m', (string) file_get_contents(variable('root_dir')."/{$dockerfile}"), $pinned);
    if ([] === $pinned[1]) {
        throw new \RuntimeException(\sprintf('%s has no RUST_VERSION.', $dockerfile));
    }
    if (!preg_match('{^\[pkg\.rust\]\nversion = "(\S+) }m', fetch('https://static.rust-lang.org/dist/channel-rust-stable.toml'), $rust)) {
        throw new \RuntimeException('The stable channel of Rust names no version of Rust.');
    }

    // The page lists its archives for scripts, in a comment:
    // PRODUCT,<version>,<year>/<archive>,<size>,<SHA3-256>
    if (!preg_match('{^PRODUCT,([\d.]+),\d+/sqlite-autoconf-\d+\.tar\.gz,}m', fetch('https://www.sqlite.org/download.html'), $sqlite)) {
        throw new \RuntimeException('sqlite.org names no release of SQLite.');
    }

    $outdated = [];
    if (version_compare(\static_php\PHP_RELEASE, $php, '<')) {
        $outdated[] = \sprintf('PHP %s is out: PHP_RELEASE is %s, in .castor/static-php.php (see docs/development.md#the-version-of-php).', $php, \static_php\PHP_RELEASE);
    }
    foreach (array_unique($pinned[1]) as $version) {
        if (version_compare($version, $rust[1], '<')) {
            $outdated[] = \sprintf('Rust %s is out: RUST_VERSION is %s, in %s.', $rust[1], $version, $dockerfile);
        }
    }

    if (version_compare(\static_php\SQLITE_RELEASE, $sqlite[1], '<')) {
        $outdated[] = \sprintf('SQLite %s is out: SQLITE_RELEASE is %s, in .castor/static-php.php (see docs/development.md#the-version-of-sqlite).', $sqlite[1], \static_php\SQLITE_RELEASE);
    }

    if ([] === $outdated) {
        io()->writeln(\sprintf('<info>✔</info> PHP %s, Rust %s and SQLite %s are the latest.', $php, $rust[1], $sqlite[1]));

        return 0;
    }

    foreach ($outdated as $message) {
        io()->writeln("<error>✘</error> {$message}");
    }

    return 1;
}

function fetch(string $url): string
{
    $response = http_request('GET', $url, ['max_duration' => 60, 'timeout' => 20]);
    if (200 !== $response->getStatusCode()) {
        throw new \RuntimeException(\sprintf('%s answered %d.', $url, $response->getStatusCode()));
    }

    return $response->getContent();
}

/**
 * @return list<string>
 */
function tag(?string $tag, string $version): array
{
    if (null === $tag) {
        return [];
    }

    if (!preg_match(TAG, $tag, $matches)) {
        return [\sprintf('The tag %s is not vX.Y.Z, with an optional -alphaN, -betaN or -rcN.', $tag)];
    }

    return same("The tag {$tag}", $matches[1], $version);
}

/**
 * @return list<string>
 */
function crates(string $version): array
{
    $root = variable('root_dir');

    $tauri = json_decode((string) file_get_contents("{$root}/crates/nativella/tauri.conf.json"), true, flags: \JSON_THROW_ON_ERROR);
    $mismatches = same('crates/nativella/tauri.conf.json: version', $tauri['version'] ?? null, $version);

    // The crates of the workspace have no source: the ones of crates.io do
    preg_match_all('{^name = "([^"]+)"\nversion = "([^"]+)"\n(?!source = )}m', (string) file_get_contents("{$root}/Cargo.lock"), $packages, \PREG_SET_ORDER);
    if ([] === $packages) {
        $mismatches[] = 'Cargo.lock lists no crate of the workspace.';
    }
    foreach ($packages as [, $name, $locked]) {
        array_push($mismatches, ...same("Cargo.lock: {$name}", $locked, $version));
    }

    return $mismatches;
}

/**
 * The packages and the demos: before 1.0, a caret constraint stays on its
 * minor version, which the branch alias of main and the path repositories
 * name too.
 *
 * @return list<string>
 */
function composer_projects(string $version): array
{
    [$major, $minor] = explode('.', $version);
    $alias = "{$major}.{$minor}.x-dev";
    $constraint = "^{$major}.{$minor}";

    $mismatches = [];
    foreach (\project\all() as $project) {
        if (!$project->isPackage() && !$project->isDemo()) {
            continue;
        }

        $file = "{$project->path}/composer.json";
        $composer = json_decode((string) file_get_contents($project->directory().'/composer.json'), true, flags: \JSON_THROW_ON_ERROR);

        if ($project->isPackage()) {
            array_push($mismatches, ...same("{$file}: extra.branch-alias.dev-main", $composer['extra']['branch-alias']['dev-main'] ?? null, $alias));
        }

        foreach ($composer['require'] ?? [] as $package => $required) {
            if (str_starts_with($package, 'nativella/')) {
                array_push($mismatches, ...same("{$file}: require.{$package}", $required, $project->isDemo() ? "{$constraint}@dev" : $constraint));
            }
        }

        foreach ($composer['repositories'] ?? [] as $repository) {
            foreach ($repository['options']['versions'] ?? [] as $package => $pathVersion) {
                array_push($mismatches, ...same("{$file}: the version of {$package} in its path repository", $pathVersion, $alias));
            }
        }
    }

    return $mismatches;
}

/**
 * @return list<string>
 */
function workflows_composer(): array
{
    $root = variable('root_dir');
    $dockerfile = 'infrastructure/docker/services/php/Dockerfile';

    if (!preg_match('{^FROM composer/composer:([^@\s]+)}m', (string) file_get_contents("{$root}/{$dockerfile}"), $matches)) {
        return ["{$dockerfile} has no composer/composer image."];
    }

    $mismatches = [];
    foreach (finder()->in("{$root}/.github/workflows")->files()->name('*.yml')->sortByName() as $workflow) {
        preg_match_all('{^\s*tools: composer:([^@\s]+)}m', $workflow->getContents(), $tools);
        foreach ($tools[1] as $composer) {
            array_push($mismatches, ...same(".github/workflows/{$workflow->getFilename()}: the Composer of setup-php", $composer, $matches[1]));
        }
    }

    return $mismatches;
}

/**
 * @return list<string> what to say when $actual is not $expected
 */
function same(string $what, mixed $actual, string $expected): array
{
    if ($actual === $expected) {
        return [];
    }

    return [\sprintf('%s is %s, not %s.', $what, \is_string($actual) ? $actual : 'missing', $expected)];
}
