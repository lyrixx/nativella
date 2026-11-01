<?php

namespace static_php;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use Symfony\Component\Process\ExecutableFinder;

use function Castor\capture;
use function Castor\context;
use function Castor\fs;
use function Castor\io;
use function Castor\variable;
use function docker\build as build_image;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\run_in_builder;

/** static-php-cli, which builds PHP and the libraries it needs */
const SPC_VERSION = '2.8.5';

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
 * About 5 minutes, then nothing until the version or the extensions change.
 */
#[AsTask(namespace: 'static-php', description: 'Builds PHP as a static library, for executables that need no PHP on the machine')]
function build(
    #[AsOption(description: 'Build again, even if nothing changed')]
    bool $force = false,
): void {
    io()->title('Building PHP '.PHP_RELEASE.' as a static library');

    // In the build, not in the cache of Castor: the CI caches the build
    $buildroot = variable('root_dir').'/static-php/buildroot';
    $stamp = "{$buildroot}/.nativella-build";
    $build = SPC_VERSION.' '.PHP_RELEASE.' '.builder_platform().' '.EXTENSIONS;
    if ([] !== spc_environment()) {
        $build .= ' gcc glibc '.GLIBC_VERSION;
    }

    // A build of Nativella 0.2.0 or older lacks the licenses of what php-src
    // bundles: the third-party notices need them
    $notices = is_file("{$buildroot}/license/php-src/README.REDIST.BINS");

    if (!$force && $notices && is_file($stamp) && file_get_contents($stamp) === $build) {
        io()->comment('Static PHP is up to date: --force builds it again.');
    } else {
        // static-php-cli keeps the libraries it finds built: they may have
        // been built for another glibc
        fs()->remove([$buildroot, variable('root_dir').'/static-php/source']);
        build_php();
        fs()->dumpFile($stamp, $build);
    }

    io()->success('Static PHP is ready in static-php/buildroot/.');
}

/**
 * The environment of a cargo command that links the static PHP, for the
 * commands of the static service (SERVICE).
 *
 * @return array<string, string>
 */
function environment(): array
{
    if (!is_file(variable('root_dir').'/static-php/buildroot/lib/libphp.a')) {
        throw new \RuntimeException('There is no static PHP yet: run "castor static-php:build" first.');
    }

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

    if (!context()['run_on_host']) {
        build_image(service: SERVICE);
    }

    spc(SERVICE);
    $environment = spc_environment();

    run_in_builder(['./spc', 'doctor', '--auto-fix'], $workDir, $environment, SERVICE);
    download(['./spc', 'download', '--for-extensions='.EXTENSIONS, '--prefer-pre-built'], $workDir, $environment, SERVICE);
    // It dumps the licenses of PHP and of the libraries it builds into
    // buildroot/license/, for the third-party notices (notices\desktop())
    run_in_builder(['./spc', 'build', EXTENSIONS, '--build-embed'], $workDir, $environment, SERVICE);
    \notices\copy_php_src_licenses('static-php/source/php-src', 'static-php/buildroot/license/php-src');
}

/**
 * Runs `spc download`, for the sources of PHP_RELEASE, into
 * static-php/downloads/. static-php-cli keeps the sources it downloaded
 * already, whatever the version asked: those of another version are
 * downloaded again.
 *
 * @param list<string>          $command     spc download and its arguments, but --with-php
 * @param array<string, string> $environment
 */
function download(array $command, string $workDir, array $environment = [], ?string $service = null): void
{
    $source = 'php-'.PHP_RELEASE.'.tar.xz';

    $command[] = '--with-php='.PHP_RELEASE;
    if (null !== downloaded_php_source() && $source !== downloaded_php_source()) {
        $command[] = '--ignore-cache-sources=php-src';
    }

    run_in_builder($command, $workDir, $environment, $service);

    if ($source !== downloaded_php_source()) {
        throw new \RuntimeException(\sprintf('static-php-cli downloaded %s, not %s: see static-php/downloads/.lock.json.', downloaded_php_source() ?? 'no PHP', $source));
    }
}

/**
 * The archive of PHP that static-php-cli downloaded, from its lock: php.net
 * names it after the version, php-8.5.12.tar.xz.
 */
function downloaded_php_source(): ?string
{
    $lock = variable('root_dir').'/static-php/downloads/.lock.json';
    if (!is_file($lock)) {
        return null;
    }

    return json_decode((string) file_get_contents($lock), true, flags: \JSON_THROW_ON_ERROR)['php-src']['filename'] ?? null;
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
