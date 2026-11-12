<?php

namespace static_php;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use Symfony\Component\Process\ExecutableFinder;

use function Castor\capture;
use function Castor\context;
use function Castor\finder;
use function Castor\fs;
use function Castor\io;
use function Castor\variable;
use function docker\build as build_image;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\run_in_builder;

/** static-php-cli, which builds PHP and the libraries it needs */
const SPC_VERSION = '2.8.5';
// Keep in sync with src/globals/defines.php of static-php-cli
const SPC_DOWNLOAD_PRE_BUILT = 2;

/**
 * The SHA-256 of the static-php-cli of each platform, as its release gives
 * them: `gh release view <SPC_VERSION> --repo crazywhalecc/static-php-cli
 * --json assets --jq '.assets[] | .name + " " + .digest'`.
 */
const SPC_SHA256 = [
    'linux-aarch64' => '675a3840dcdc4ed041fe20eaa54310ce019a9984c1c03951df9ec66df5795213',
    'linux-x86_64' => '523ba4279c54c7a377156c0dd3a36adf92ee64b01e9a7f5e9e2ec084b8e458e5',
    'macos-aarch64' => 'acf2f25d56d0cbf8e65aa82e5054fef555f7be7c5c38046c6e0819f266d83225',
    'macos-x86_64' => 'e8b798048f62ca4960764196543b60ae703f7174aa418824cf542aeec1d2cd6a',
];

/**
 * The extensions Symfony, Laravel and Doctrine applications need, on the
 * desktop and on the phones (.castor/static-php-mobile.php, which says how to
 * build each one there). Keep in sync with docs/building.md and docs/mobile.md.
 */
const EXTENSIONS = 'bcmath,ctype,curl,dom,fileinfo,filter,iconv,intl,mbstring,opcache,openssl,pdo,pdo_sqlite,phar,session,simplexml,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,zip,zlib';

/**
 * The release of PHP, to its patch version: every platform builds this one,
 * and builds again when it changes. Raise it with each release of PHP, the
 * security ones first: see docs/development.md#the-version-of-php.
 */
const PHP_RELEASE = '8.5.12';

/**
 * The SHA-256 of php-<PHP_RELEASE>.tar.xz, as php.net gives it:
 * https://www.php.net/releases/index.php?json&version=<PHP_RELEASE>. Raise it
 * with PHP_RELEASE.
 */
const PHP_RELEASE_SHA256 = '08bc2eba2a588654ade1357174aa646b761556c862dda0678345ee29ed481209';

/**
 * The release of SQLite: static-php-cli pins 3.45.2, from 2024, so every
 * platform builds this one instead, and builds again when it changes. Raise
 * it with each release of SQLite, with the two constants below: see
 * docs/development.md#the-version-of-sqlite.
 */
const SQLITE_RELEASE = '3.54.1';

/** The year of SQLITE_RELEASE: the directory of its archive on sqlite.org. */
const SQLITE_RELEASE_YEAR = '2026';

/**
 * The SHA3-256 of the archive of SQLITE_RELEASE (sqlite_archive()), as
 * https://www.sqlite.org/download.html gives it.
 */
const SQLITE_RELEASE_SHA3_256 = 'b6ced585c750916b1052667239a12c25a7ff34e7f06e1e6e01ecb0427657d9c4';

/**
 * The extensions of SQLite that the configure script of 3.45.2 enabled by
 * itself, and that the one of SQLITE_RELEASE leaves out unless asked: on the
 * desktop and on the phones.
 */
const SQLITE_OPTIONS = ['--enable-fts4', '--enable-fts5', '--enable-geopoly', '--enable-rtree'];

/**
 * The service that builds the static PHP and links it: Ubuntu 22.04, for an
 * old glibc.
 */
const SERVICE = 'static';

/**
 * The glibc of the static service: the Linux executables run on the
 * distributions with this one or a newer one. Keep in sync with the static
 * stage of infrastructure/docker/services/php/Dockerfile.
 */
const GLIBC_VERSION = '2.35';

/**
 * The newest symbol version of the libstdc++ of the static service (GCC 12):
 * the Linux executables need it for intl and ICU, which are C++, and run on
 * the distributions with this one or a newer one. Keep in sync with the
 * static stage of infrastructure/docker/services/php/Dockerfile.
 */
const GLIBCXX_VERSION = '3.4.30';

/**
 * PHP as a static library (embed SAPI), into static-php/buildroot/: lib/
 * with libphp.a and its dependencies, include/php/, bin/php-config. Linked
 * against it (nativella:build, and --static on the other tasks that build
 * the Rust crates), the nativella command and the executables it builds need no PHP on the
 * machine. In the static service, or on the host in the host context, as on
 * macOS.
 *
 * About 5 minutes, then nothing until the versions or the extensions change.
 * static-php-cli downloads the latest release of each library PHP links, but
 * SQLite, which SQLITE_RELEASE pins: the stamp of the build records them, and
 * --update-libraries builds again when one of them has a newer one, as the CI
 * asks on main and for a release.
 */
#[AsTask(namespace: 'static-php', description: 'Builds PHP as a static library, for executables that need no PHP on the machine')]
function build(
    #[AsOption(description: 'Build again, even if nothing changed')]
    bool $force = false,
    #[AsOption(description: 'Build again if a library PHP links has a newer release than the one it was built with')]
    bool $updateLibraries = false,
): void {
    io()->title('Building PHP '.PHP_RELEASE.' as a static library');

    // In the build, not in the cache of Castor: the CI caches the build
    $buildroot = variable('root_dir').'/static-php/buildroot';
    $stamp = "{$buildroot}/.nativella-build";
    $build = SPC_VERSION.' '.PHP_RELEASE.' sqlite-'.SQLITE_RELEASE.' '.builder_platform().' '.EXTENSIONS;
    if ([] !== spc_environment()) {
        $build .= ' gcc glibc '.GLIBC_VERSION;
    }

    // A build of Nativella 0.2.0 or older lacks the licenses of what php-src
    // bundles: the third-party notices need them
    $notices = is_file("{$buildroot}/license/php-src/README.REDIST.BINS");

    $recorded = stamped_sources($stamp);
    $upToDate = !$force && $notices && is_file($stamp) && file_get_contents($stamp) === stamp($build, $recorded);
    if ($upToDate && $updateLibraries) {
        prepare();
        download_for_extensions(array_keys($recorded));
        $upToDate = downloaded_sources(array_keys($recorded)) === $recorded;
    }

    if ($upToDate) {
        io()->comment('Static PHP is up to date: --force builds it again.');
    } else {
        // static-php-cli keeps the libraries it finds built: they may have
        // been built for another glibc
        fs()->remove([$buildroot, variable('root_dir').'/static-php/source']);
        build_php();
        fs()->dumpFile($stamp, stamp($build, downloaded_sources(linked_sources('static-php/buildroot/license'))));
    }

    io()->success('Static PHP is ready in static-php/buildroot/.');
}

/**
 * The environment of a cargo command that links the static PHP, for the
 * commands of the static service (SERVICE): build() it first.
 *
 * @return array<string, string>
 */
function environment(): array
{
    $buildroot = builder_path('static-php/buildroot');

    return [
        // Apart from the builds linking the PHP of the system: cargo would
        // reuse the bindings generated for one with the other, and the
        // objects compiled against the glibc of the builder
        'CARGO_TARGET_DIR' => builder_path(context()['run_on_host'] ? 'target/static' : 'target/docker-static'),
        // ext-php-rs reads `php -i` and php-config. An embed-only build has
        // no php binary: the stand-in of mobile/php describes it from its
        // headers.
        'NATIVELLA_PHP_DIR' => $buildroot,
        // The runtime links every static library of lib/ (crates/runtime/build.rs)
        'NATIVELLA_PHP_LIB_DIR' => "{$buildroot}/lib",
        'PHP' => builder_path('mobile/php/php'),
        'PHP_CONFIG' => "{$buildroot}/bin/php-config",
    ];
}

/**
 * The environment of a cargo command that links the PHP of the system,
 * dynamically: none. The builder has that PHP; on the host, it must be
 * installed.
 *
 * @return array<string, string>
 */
function system_php_environment(): array
{
    if (context()['run_on_host'] && $missing = missing_system_php()) {
        throw new \RuntimeException(\sprintf("Linking the PHP of the system needs, on this machine (see docs/development.md#the-php-of-the-system):\n - %s\nOr link the static PHP instead: \"castor static-php:build\", then the same task with --static.", implode("\n - ", $missing)));
    }

    return [];
}

/**
 * What ext-php-rs needs to link the PHP of the system: php (`php -i`),
 * php-config, from PATH or from the PHP and PHP_CONFIG variables as it reads
 * them, and libphp next to the other libraries of that PHP.
 *
 * @return list<string> what is missing, and how to get it
 */
function missing_system_php(): array
{
    $finder = new ExecutableFinder();
    $missing = [];

    $php = getenv('PHP') ?: $finder->find('php');
    if (null === $php || !is_executable($php)) {
        $missing[] = 'The php command (php8.5-cli on Ubuntu), or PHP pointing to it.';
    }

    $phpConfig = getenv('PHP_CONFIG') ?: $finder->find('php-config');
    if (null === $phpConfig || !is_executable($phpConfig)) {
        $missing[] = 'The php-config command and the headers of PHP (php8.5-dev on Ubuntu), or PHP_CONFIG pointing to it.';

        return $missing;
    }

    $libDir = trim(capture([$phpConfig, '--prefix'])).'/lib';
    if (!is_file("{$libDir}/libphp.so") && !is_file("{$libDir}/libphp.dylib")) {
        $missing[] = \sprintf('libphp, PHP as a shared library, in %s (libphp8.5-embed on Ubuntu).', $libDir);
    }

    return $missing;
}

/**
 * On Linux, static-php-cli builds with the gcc of the static service
 * (SPC_LIBC=glibc), not with zig, its default: against the glibc of the
 * service, as the rest of the application (WebKitGTK) is, and for the
 * baseline of the architecture, not for the processor of the build machine:
 * the executables would stop with an illegal instruction on an older one. gcc
 * builds intl and ICU, which are C++, against the libstdc++ that WebKitGTK
 * loads too; zig would build them against a libc++ of its own, which the
 * linker of cargo does not have.
 *
 * @return array<string, string>
 */
function spc_environment(): array
{
    if (!str_starts_with(builder_platform(), 'linux-')) {
        return [];
    }

    return ['SPC_LIBC' => 'glibc'];
}

function build_php(): void
{
    $workDir = builder_path('static-php');
    $environment = spc_environment();

    prepare();
    run_in_builder(['./spc', 'doctor', '--auto-fix'], $workDir, $environment, SERVICE);
    download_for_extensions();
    if (downloaded_sqlite()) {
        configure_sqlite($workDir, $environment);
    }
    // It dumps the licenses of PHP and of the libraries it builds into
    // buildroot/license/, for the third-party notices (notices\desktop())
    run_in_builder(['./spc', 'build', EXTENSIONS, '--build-embed'], $workDir, $environment, SERVICE);
    \notices\copy_php_src_licenses('static-php/source/php-src', 'static-php/buildroot/license/php-src');
}

/**
 * static-php-cli configures SQLite as it does the 3.45.2 it pins, whose
 * configure script came from autoconf. The one of SQLITE_RELEASE is another
 * tool, autosetup: it stops at the options it does not know, --with-pic and
 * --enable-pic, and builds without the extensions of SQLITE_OPTIONS. It also
 * finds the math functions in the LIBS of static-php-cli, so it links its
 * shell, which make builds too, without libm: with an empty LIBS, it looks
 * for the library itself. It is a wrapper, which hands its arguments over:
 * it now adds these ones.
 *
 * static-php-cli keeps a source it finds extracted from the archive of its
 * lock, so the build that follows configures this one.
 *
 * @param array<string, string> $environment
 */
function configure_sqlite(string $workDir, array $environment): void
{
    run_in_builder(['./spc', 'extract', 'sqlite', '--source-only'], $workDir, $environment, SERVICE);

    $configure = variable('root_dir').'/static-php/source/sqlite/configure';
    $contents = (string) file_get_contents($configure);
    $arguments = ' "$@"';
    if (1 !== substr_count($contents, $arguments)) {
        throw new \RuntimeException(\sprintf('%s does not hand its arguments over with %s any more: the configure script of SQLite changed, check how to give it %s.', $configure, trim($arguments), implode(' ', SQLITE_OPTIONS)));
    }

    fs()->dumpFile($configure, str_replace($arguments, $arguments.' --disable-option-checking LIBS= '.implode(' ', SQLITE_OPTIONS), $contents));
}

/**
 * What static-php-cli runs in: the static service, built, and the tool.
 */
function prepare(): void
{
    if (!context()['run_on_host']) {
        build_image(service: SERVICE);
    }

    spc(SERVICE);
}

/**
 * Downloads the sources of PHP and of the libraries of EXTENSIONS.
 *
 * @param list<string> $refresh the sources to download again, in their latest release
 */
function download_for_extensions(array $refresh = []): void
{
    download(['./spc', 'download', '--for-extensions='.EXTENSIONS, '--prefer-pre-built'], builder_path('static-php'), spc_environment(), SERVICE, $refresh);
}

/**
 * The stamp of a build: what it was built from, then each source static-php-cli
 * downloaded for it, "<source> <archive> <hash>".
 *
 * @param array<string, string> $sources "<archive> <hash>" by source (downloaded_sources())
 */
function stamp(string $identity, array $sources): string
{
    $stamp = "{$identity}\n";
    foreach ($sources as $source => $archive) {
        $stamp .= "{$source} {$archive}\n";
    }

    return $stamp;
}

/**
 * The sources a stamp records: none for a build older than them, which
 * builds again.
 *
 * @return array<string, string> "<archive> <hash>" by source
 */
function stamped_sources(string $stamp): array
{
    $sources = [];
    foreach (\array_slice(is_file($stamp) ? file($stamp, \FILE_IGNORE_NEW_LINES) ?: [] : [], 1) as $line) {
        [$source, $archive] = explode(' ', $line, 2);
        $sources[$source] = $archive;
    }

    return $sources;
}

/**
 * The archive of each source, as static-php-cli downloaded it last, and its
 * hash, which tells two releases apart when the archive has the same name.
 *
 * @param list<string> $sources the names of static-php-cli
 *
 * @return array<string, string> "<archive> <hash>" by source, sorted
 */
function downloaded_sources(array $sources): array
{
    $lock = json_decode((string) file_get_contents(variable('root_dir').'/static-php/downloads/.lock.json'), true, flags: \JSON_THROW_ON_ERROR);

    $archives = [];
    foreach ($sources as $source) {
        $entry = $lock[$source] ?? pre_built_library($lock, $source) ?? throw new \RuntimeException(\sprintf('static-php-cli downloaded no %s: see static-php/downloads/.lock.json.', $source));
        // A source cloned with Git has a directory, not an archive
        $archives[$source] = ($entry['filename'] ?? $entry['dirname']).' '.$entry['hash'];
    }
    ksort($archives);

    return $archives;
}

/**
 * The entry of a library static-php-cli downloaded built, as --prefer-pre-built
 * asks where there is one, on macOS: its lock names it
 * "<source>-<OS family>-<architecture>-<libc>-<version of the libc>".
 *
 * @param array<string, array{filename?: string, dirname?: string, hash: string, lock_as: int}> $lock
 *
 * @return ?array{filename?: string, dirname?: string, hash: string, lock_as: int}
 */
function pre_built_library(array $lock, string $source): ?array
{
    foreach ($lock as $name => $entry) {
        if (SPC_DOWNLOAD_PRE_BUILT === $entry['lock_as'] && str_starts_with($name, "{$source}-".\PHP_OS_FAMILY.'-')) {
            return $entry;
        }
    }

    return null;
}

/**
 * The sources of the libraries and of the extensions PHP links, php-src
 * aside, which PHP_RELEASE pins: static-php-cli dumped a license for each,
 * lib_<source>_<index>.txt or ext_<name>_<index>.txt, for the source ext-<name>.
 *
 * @param string $licenses from the root of the repository
 *
 * @return list<string>
 */
function linked_sources(string $licenses): array
{
    $sources = [];
    foreach (finder()->in(variable('root_dir')."/{$licenses}")->files()->depth(0)->name('*.txt') as $file) {
        if (preg_match('/^(ext|lib)_(.+)_\d+\.txt$/', $file->getFilename(), $matches)) {
            $sources[] = 'ext' === $matches[1] ? "ext-{$matches[2]}" : $matches[2];
        }
    }

    return array_values(array_unique($sources));
}

/**
 * Runs `spc download`, for the sources of PHP_RELEASE and of SQLITE_RELEASE,
 * into static-php/downloads/. static-php-cli keeps the sources it downloaded
 * already, whatever the version asked: those of another version of PHP are
 * downloaded again, and SQLite, from a URL of ours, each time it is among the
 * sources.
 *
 * @param list<string>          $command     spc download and its arguments, but --with-php and --custom-url
 * @param array<string, string> $environment
 * @param list<string>          $refresh     the sources to download again, in their latest release
 */
function download(array $command, string $workDir, array $environment = [], ?string $service = null, array $refresh = []): void
{
    $source = 'php-'.PHP_RELEASE.'.tar.xz';

    $command[] = '--with-php='.PHP_RELEASE;
    $command[] = \sprintf('--custom-url=sqlite:https://www.sqlite.org/%s/%s', SQLITE_RELEASE_YEAR, sqlite_archive());
    if (null !== downloaded_archive('php-src') && $source !== downloaded_archive('php-src')) {
        $refresh[] = 'php-src';
    }
    if ([] !== $refresh) {
        $command[] = '--ignore-cache-sources='.implode(',', $refresh);
    }

    // It asks the GitHub API where some sources are: a token raises its rate
    // limit. Given to this command only, not to the builds
    $token = getenv('GITHUB_TOKEN');
    if (false !== $token && '' !== $token) {
        $environment['GITHUB_TOKEN'] = $token;
    }

    run_in_builder($command, $workDir, $environment, $service);

    if ($source !== downloaded_archive('php-src')) {
        throw new \RuntimeException(\sprintf('static-php-cli downloaded %s, not %s: see static-php/downloads/.lock.json.', downloaded_archive('php-src') ?? 'no PHP', $source));
    }

    $sha256 = hash_file('sha256', variable('root_dir')."/static-php/downloads/{$source}");
    if (PHP_RELEASE_SHA256 !== $sha256) {
        throw new \RuntimeException(\sprintf('static-php/downloads/%s has the SHA-256 %s, not the %s of PHP_RELEASE_SHA256.', $source, $sha256, PHP_RELEASE_SHA256));
    }

    if (downloaded_sqlite()) {
        $sha3 = hash_file('sha3-256', variable('root_dir').'/static-php/downloads/'.sqlite_archive());
        if (SQLITE_RELEASE_SHA3_256 !== $sha3) {
            throw new \RuntimeException(\sprintf('static-php/downloads/%s has the SHA3-256 %s, not the %s of SQLITE_RELEASE_SHA3_256.', sqlite_archive(), $sha3, SQLITE_RELEASE_SHA3_256));
        }
    }
}

/**
 * The archive of SQLITE_RELEASE, as sqlite.org names it:
 * sqlite-autoconf-3540100.tar.gz for 3.54.1.
 */
function sqlite_archive(): string
{
    return vsprintf('sqlite-autoconf-%d%02d%02d00.tar.gz', explode('.', SQLITE_RELEASE));
}

/**
 * Whether static-php-cli downloaded SQLITE_RELEASE, from its lock: only when
 * an extension of EXTENSIONS needs SQLite.
 */
function downloaded_sqlite(): bool
{
    return sqlite_archive() === downloaded_archive('sqlite');
}

/**
 * The archive of $source that static-php-cli downloaded last, from its lock:
 * php.net names the one of PHP after its version, php-8.5.12.tar.xz.
 */
function downloaded_archive(string $source): ?string
{
    $lock = variable('root_dir').'/static-php/downloads/.lock.json';
    if (!is_file($lock)) {
        return null;
    }

    return json_decode((string) file_get_contents($lock), true, flags: \JSON_THROW_ON_ERROR)[$source]['filename'] ?? null;
}

/**
 * static-php-cli, for the platform of the builder, downloaded once.
 *
 * @param ?string $service the service that unpacks it: the builder by default
 *
 * @return string from the root of the repository
 */
function spc(?string $service = null): string
{
    $dir = variable('root_dir').'/static-php';

    if (!is_file("{$dir}/spc")) {
        $platform = builder_platform();
        $sha256 = SPC_SHA256[$platform] ?? throw new \RuntimeException(\sprintf('static-php-cli has no build for %s.', $platform));
        \download\verified(\sprintf('https://github.com/crazywhalecc/static-php-cli/releases/download/%s/spc-%s.tar.gz', SPC_VERSION, $platform), "{$dir}/spc.tar.gz", $sha256);
        run_in_builder(['tar', 'xzf', 'spc.tar.gz'], builder_path('static-php'), service: $service);
        fs()->remove("{$dir}/spc.tar.gz");
    }

    return 'static-php/spc';
}
