<?php

namespace static_php;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;

use function Castor\context;
use function Castor\fs;
use function Castor\http_download;
use function Castor\io;
use function Castor\variable;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\run_in_builder;

/** static-php-cli, which builds PHP and the libraries it needs */
const SPC_VERSION = '2.8.5';

/**
 * The extensions Symfony, Laravel and Doctrine applications need. Keep in
 * sync with docs/building.md.
 */
const EXTENSIONS = 'bcmath,ctype,curl,dom,fileinfo,filter,iconv,mbstring,opcache,openssl,pdo,pdo_sqlite,phar,session,simplexml,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,zip,zlib';

const PHP_BRANCH = '8.5';

/**
 * PHP as a static library (embed SAPI), into static-php/buildroot/: lib/
 * with libphp.a and its dependencies, include/php/, bin/php-config. Linked
 * against it (--static on the tasks that build the Rust crates), the
 * nativella command and the executables it builds need no PHP on the
 * machine. In the builder, or on the host in the host context, as on macOS.
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
    $stamp = variable('root_dir').'/static-php/buildroot/.nativella-build';
    $build = SPC_VERSION.' '.PHP_BRANCH.' '.builder_platform().' '.EXTENSIONS;

    if (!$force && is_file($stamp) && file_get_contents($stamp) === $build) {
        io()->comment('Static PHP is up to date: --force builds it again.');
    } else {
        build_php();
        fs()->dumpFile($stamp, $build);
    }

    io()->success('Static PHP is ready in static-php/buildroot/.');
}

/**
 * The environment of a cargo command that links the static PHP, for the
 * commands of the builder.
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
        // reuse the bindings generated for one with the other
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

function build_php(): void
{
    $dir = variable('root_dir').'/static-php';
    $workDir = builder_path('static-php');

    if (!is_file("{$dir}/spc")) {
        http_download(\sprintf('https://github.com/crazywhalecc/static-php-cli/releases/download/%s/spc-%s.tar.gz', SPC_VERSION, builder_platform()), "{$dir}/spc.tar.gz");
        run_in_builder(['tar', 'xzf', 'spc.tar.gz'], $workDir);
        fs()->remove("{$dir}/spc.tar.gz");
    }

    // On Linux, linked against the glibc of the system, as the rest of the
    // application (WebKitGTK) is: static-php-cli builds with zig for that
    $environment = str_starts_with(builder_platform(), 'linux-') ? ['SPC_TARGET' => 'native-native-gnu'] : [];

    run_in_builder(['./spc', 'doctor', '--auto-fix'], $workDir, $environment);
    run_in_builder(['./spc', 'download', '--for-extensions='.EXTENSIONS, '--with-php='.PHP_BRANCH, '--prefer-pre-built'], $workDir, $environment);
    run_in_builder(['./spc', 'build', EXTENSIONS, '--build-embed'], $workDir, $environment);
}
