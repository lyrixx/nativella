<?php

namespace static_php;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use Symfony\Component\Process\ExecutableFinder;

use function Castor\capture;
use function Castor\context;
use function Castor\fs;
use function Castor\http_download;
use function Castor\io;
use function Castor\variable;
use function docker\build as build_image;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\run_in_builder;

/** static-php-cli, which builds PHP and the libraries it needs */
const SPC_VERSION = '2.8.5';

/**
 * The extensions Symfony, Laravel and Doctrine applications need. Keep in
 * sync with docs/building.md.
 */
const EXTENSIONS = 'bcmath,ctype,curl,dom,fileinfo,filter,iconv,intl,mbstring,opcache,openssl,pdo,pdo_sqlite,phar,session,simplexml,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,zip,zlib';

const PHP_BRANCH = '8.5';

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
    io()->title('Building PHP '.PHP_BRANCH.' as a static library');

    // In the build, not in the cache of Castor: the CI caches the build
    $buildroot = variable('root_dir').'/static-php/buildroot';
    $stamp = "{$buildroot}/.nativella-build";
    $build = SPC_VERSION.' '.PHP_BRANCH.' '.builder_platform().' '.EXTENSIONS;
    if ([] !== spc_environment()) {
        $build .= ' gcc glibc '.GLIBC_VERSION;
    }

    if (!$force && is_file($stamp) && file_get_contents($stamp) === $build) {
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
    $dir = variable('root_dir').'/static-php';
    $workDir = builder_path('static-php');

    if (!context()['run_on_host']) {
        build_image(service: SERVICE);
    }

    if (!is_file("{$dir}/spc")) {
        http_download(\sprintf('https://github.com/crazywhalecc/static-php-cli/releases/download/%s/spc-%s.tar.gz', SPC_VERSION, builder_platform()), "{$dir}/spc.tar.gz");
        run_in_builder(['tar', 'xzf', 'spc.tar.gz'], $workDir, service: SERVICE);
        fs()->remove("{$dir}/spc.tar.gz");
    }

    $environment = spc_environment();

    run_in_builder(['./spc', 'doctor', '--auto-fix'], $workDir, $environment, SERVICE);
    run_in_builder(['./spc', 'download', '--for-extensions='.EXTENSIONS, '--with-php='.PHP_BRANCH, '--prefer-pre-built'], $workDir, $environment, SERVICE);
    run_in_builder(['./spc', 'build', EXTENSIONS, '--build-embed'], $workDir, $environment, SERVICE);
}
