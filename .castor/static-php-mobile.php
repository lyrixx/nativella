<?php

namespace static_php\mobile;

use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use Symfony\Component\Process\ExecutableFinder;

use function Castor\capture;
use function Castor\context;
use function Castor\finder;
use function Castor\fs;
use function Castor\io;
use function Castor\run;
use function Castor\variable;
use function docker\builder_path;
use function docker\docker_compose_run;
use function docker\run_in_builder;
use function mobile\ndk_home;
use function static_php\spc;

use const static_php\EXTENSIONS;
use const static_php\PHP_BRANCH;
use const static_php\SPC_VERSION;

/**
 * How PHP's configure enables each extension of the desktop
 * (static_php\EXTENSIONS) on a phone, and the libraries it needs, from
 * LIBRARIES: static-php-cli builds PHP for computers only. An extension added
 * to EXTENSIONS needs its options here, and a recipe in build_library() for
 * each new library.
 *
 * Opcache is always built since PHP 8.5: it has no option.
 *
 * @var array<string, array{options: list<string>, libraries?: list<string>}>
 */
const CONFIGURE = [
    'bcmath' => ['options' => ['--enable-bcmath']],
    'ctype' => ['options' => ['--enable-ctype']],
    'curl' => ['options' => ['--with-curl'], 'libraries' => ['curl']],
    'dom' => ['options' => ['--enable-dom', '--with-libxml'], 'libraries' => ['libxml2']],
    'fileinfo' => ['options' => ['--enable-fileinfo']],
    'filter' => ['options' => ['--enable-filter']],
    'iconv' => ['options' => ['--with-iconv={prefix}'], 'libraries' => ['libiconv']],
    'intl' => ['options' => ['--enable-intl'], 'libraries' => ['icu']],
    // Without Oniguruma, as on the desktop
    'mbstring' => ['options' => ['--enable-mbstring', '--disable-mbregex']],
    'opcache' => ['options' => []],
    // With Argon2 from OpenSSL, for password_hash(), as on the desktop
    'openssl' => ['options' => ['--with-openssl', '--with-openssl-argon2'], 'libraries' => ['openssl']],
    'pdo' => ['options' => ['--enable-pdo']],
    'pdo_sqlite' => ['options' => ['--with-pdo-sqlite'], 'libraries' => ['sqlite']],
    'phar' => ['options' => ['--enable-phar']],
    'session' => ['options' => ['--enable-session']],
    'simplexml' => ['options' => ['--enable-simplexml', '--with-libxml'], 'libraries' => ['libxml2']],
    'sodium' => ['options' => ['--with-sodium'], 'libraries' => ['libsodium']],
    'sqlite3' => ['options' => ['--with-sqlite3'], 'libraries' => ['sqlite']],
    'tokenizer' => ['options' => ['--enable-tokenizer']],
    'xml' => ['options' => ['--enable-xml', '--with-libxml'], 'libraries' => ['libxml2']],
    'xmlreader' => ['options' => ['--enable-xmlreader', '--with-libxml'], 'libraries' => ['libxml2']],
    'xmlwriter' => ['options' => ['--enable-xmlwriter', '--with-libxml'], 'libraries' => ['libxml2']],
    'zip' => ['options' => ['--with-zip'], 'libraries' => ['libzip']],
    'zlib' => ['options' => ['--with-zlib'], 'libraries' => ['zlib']],
];

/**
 * The libraries of the extensions, each after the ones it needs: the order
 * they build in. Named after the sources of static-php-cli, which downloads
 * them: the versions of the desktop, built with its features.
 *
 * @var array<string, list<string>> the libraries each one needs
 */
const LIBRARIES = [
    'zlib' => [],
    'openssl' => ['zlib'],
    'libiconv' => [],
    'libxml2' => ['libiconv', 'zlib'],
    'sqlite' => [],
    'libsodium' => [],
    'libzip' => ['openssl', 'zlib'],
    'curl' => ['openssl', 'zlib'],
    'icu' => [],
];

/**
 * The iOS version PHP is built for: the default
 * bundle.iOS.minimumSystemVersion of Tauri, which builds the app.
 */
const IOS_VERSION = '15.0';

/**
 * PHP as a static library for Android, with the extensions of the desktop,
 * into static-php/android-<arch>/buildroot/: what `castor
 * mobile:android-template` links. In the android service, which has the NDK,
 * or on the host in the host context.
 *
 * About 4 minutes, then nothing until the versions, the extensions, the NDK
 * or this file change.
 */
#[AsTask(namespace: 'static-php', description: 'Builds PHP as a static library for Android, with the extensions of the desktop')]
function android(
    #[AsOption(description: 'The ABI: arm64-v8a, for phones, or x86_64, for the emulators of x86_64 computers')]
    string $abi = 'arm64-v8a',
    #[AsOption(description: 'Build again, even if nothing changed')]
    bool $force = false,
): void {
    build(Target::android($abi), $force);
}

/**
 * PHP as a static library for iOS and its simulator, with the extensions of
 * the desktop, into static-php/ios-arm64/buildroot/ and
 * static-php/ios-simulator-arm64/buildroot/: what `castor mobile:ios` links.
 * On macOS, with Xcode, whatever the context.
 *
 * EXPERIMENTAL: .github/workflows/mobile.yml runs it on a macOS runner.
 */
#[AsTask(namespace: 'static-php', description: 'Builds PHP as a static library for iOS and its simulator, with the extensions of the desktop, on macOS (experimental)')]
function ios(
    #[AsOption(description: 'Build again, even if nothing changed')]
    bool $force = false,
): void {
    foreach ([Target::iphoneos(), Target::iphonesimulator()] as $target) {
        build($target, $force);
    }
}

/**
 * Builds PHP for $target, unless it is built already from the same versions,
 * extensions, toolchain and recipes.
 *
 * @return string the buildroot, from the root of the repository: lib/ with
 *                libphp.a and the libraries it needs, include/php/,
 *                bin/php-config
 */
function build(Target $target, bool $force = false): string
{
    $root = variable('root_dir');
    $dir = "static-php/{$target->name}";
    $buildroot = "{$dir}/buildroot";
    $stamp = "{$root}/{$buildroot}/.nativella-build";

    $toolchain = toolchain($target);
    // The recipes too, which are these, not static-php-cli's
    $identity = implode(' ', [SPC_VERSION, PHP_BRANCH, $target->name, $toolchain->version, EXTENSIONS, hash_file('xxh128', __FILE__)]);

    if (!$force && is_file($stamp) && file_get_contents($stamp) === $identity) {
        return $buildroot;
    }

    io()->title(\sprintf('Building PHP %s as a static library for %s', PHP_BRANCH, $target->name));

    $libraries = libraries();
    $archives = download(['php-src', ...$libraries]);

    fs()->remove(["{$root}/{$buildroot}", "{$root}/{$dir}/source"]);
    $build = new Build($target, $toolchain, $buildroot, "{$dir}/source");

    foreach ($libraries as $library) {
        io()->section("Building {$library}");
        build_library($build, $library, $archives[$library]);
    }

    io()->section('Building PHP');
    build_php($build, $archives['php-src']);

    // The licenses of PHP and of its libraries, which static-php-cli dumps
    // from their sources as for the desktop, with the ones of what php-src
    // bundles: for the third-party notices (notices\android()). Each dump
    // empties its directory first: PHP's goes aside, then joins the others
    $dump = static fn (string $what, string $to) => run_in_builder([builder_path(spc()), 'dump-license', $what, '--dump-dir='.builder_path($to), '--no-motd'], builder_path('static-php'), ['SOURCE_PATH' => builder_path("{$dir}/source")]);
    $dump('--for-libs='.implode(',', $libraries), "{$buildroot}/license");
    $dump('--for-sources=php-src', "{$dir}/source/license-php-src");
    foreach (finder()->in("{$root}/{$dir}/source/license-php-src")->files()->name('src_php-src_*.txt') as $license) {
        fs()->rename($license->getPathname(), "{$root}/{$buildroot}/license/{$license->getFilename()}");
    }
    \notices\copy_php_src_licenses("{$dir}/source/php-src", "{$buildroot}/license/php-src");

    // The NDK builds with debug information, which the debug builds of the
    // applications would carry: the symbols are enough
    $built = iterator_to_array(finder()->in("{$root}/{$buildroot}/lib")->depth(0)->files()->name('*.a'), false);
    $build->run([...$toolchain->strip, ...array_map(static fn (\SplFileInfo $library): string => $library->getFilename(), $built)], "{$buildroot}/lib");

    fs()->remove("{$root}/{$dir}/source");
    fs()->dumpFile($stamp, $identity);

    io()->success("PHP for {$target->name} is ready in {$buildroot}/.");

    return $buildroot;
}

/**
 * The libraries the extensions of EXTENSIONS need, in the order they build.
 *
 * @return list<string>
 */
function libraries(): array
{
    $extensions = explode(',', EXTENSIONS);
    if ($missing = array_diff($extensions, array_keys(CONFIGURE))) {
        throw new \LogicException(\sprintf('No options for the phones for %s: add them to CONFIGURE in %s.', implode(', ', $missing), __FILE__));
    }

    $needed = [];
    foreach ($extensions as $extension) {
        array_push($needed, ...CONFIGURE[$extension]['libraries'] ?? []);
    }

    if ($missing = array_diff($needed, array_keys(LIBRARIES))) {
        throw new \LogicException(\sprintf('No recipe for the phones for %s: add them to LIBRARIES and build_library() in %s.', implode(', ', $missing), __FILE__));
    }

    // With the libraries they need: LIBRARIES lists them before, so that the
    // other way round, each library adds those of the ones it needs
    foreach (array_reverse(LIBRARIES) as $library => $dependencies) {
        if (\in_array($library, $needed, true)) {
            array_push($needed, ...$dependencies);
        }
    }

    return array_values(array_intersect(array_keys(LIBRARIES), $needed));
}

/**
 * Downloads the sources with static-php-cli, as the desktop build does, into
 * static-php/downloads/, unless they are there already: the versions of the
 * desktop, as long as both share these downloads.
 *
 * @param list<string> $sources the names of static-php-cli
 *
 * @return array<string, string> the archive of each source, from the root of
 *                               the repository
 */
function download(array $sources): array
{
    io()->section('Downloading the sources');

    run_in_builder([builder_path(spc()), 'download', implode(',', $sources), '--with-php='.PHP_BRANCH], builder_path('static-php'));

    $lock = json_decode((string) file_get_contents(variable('root_dir').'/static-php/downloads/.lock.json'), true, flags: \JSON_THROW_ON_ERROR);

    $archives = [];
    foreach ($sources as $source) {
        $filename = $lock[$source]['filename'] ?? throw new \RuntimeException(\sprintf('static-php-cli downloaded no archive of %s: see static-php/downloads/.lock.json.', $source));
        $archives[$source] = "static-php/downloads/{$filename}";
    }

    return $archives;
}

/**
 * Extracts $library, builds it and installs it into the buildroot, with the
 * features static-php-cli builds it with for the desktop.
 */
function build_library(Build $build, string $library, string $archive): void
{
    $source = $build->extract($archive, $library);
    $prefix = $build->path($build->buildroot);

    match ($library) {
        'zlib' => $build->zlib($source),
        'openssl' => $build->openssl($source, [
            'no-shared', 'no-tests', 'no-docs', 'no-legacy',
            'zlib', "--with-zlib-include={$prefix}/include", "--with-zlib-lib={$prefix}/lib",
            "--prefix={$prefix}", '--libdir=lib',
        ]),
        'libiconv' => $build->autoconf($source, ['--enable-extra-encodings'], ['install-lib'], ['libcharset']),
        'libxml2' => $build->cmake($source, [
            '-DLIBXML2_WITH_ICONV=ON', '-DIconv_IS_BUILT_IN=OFF',
            '-DLIBXML2_WITH_ZLIB=ON', '-DLIBXML2_WITH_LZMA=OFF',
            // Discouraged by libxml2
            '-DLIBXML2_WITH_ICU=OFF',
            '-DLIBXML2_WITH_PYTHON=OFF', '-DLIBXML2_WITH_PROGRAMS=OFF', '-DLIBXML2_WITH_TESTS=OFF',
        ]),
        // The library only: its shell runs commands with system(), which iOS
        // does not have
        'sqlite' => $build->autoconf($source, [], ['install-libLTLIBRARIES', 'install-includeHEADERS', 'install-pkgconfigDATA']),
        'libsodium' => $build->autoconf($source),
        'libzip' => $build->cmake($source, [
            '-DENABLE_OPENSSL=ON', '-DENABLE_GNUTLS=OFF', '-DENABLE_MBEDTLS=OFF',
            '-DENABLE_BZIP2=OFF', '-DENABLE_LZMA=OFF', '-DENABLE_ZSTD=OFF',
            '-DBUILD_DOC=OFF', '-DBUILD_EXAMPLES=OFF', '-DBUILD_REGRESS=OFF', '-DBUILD_TOOLS=OFF', '-DBUILD_OSSFUZZ=OFF',
        ]),
        'curl' => $build->cmake($source, [
            '-DCURL_USE_OPENSSL=ON', '-DCURL_CA_BUNDLE=OFF', '-DCURL_CA_PATH=OFF', '-DCURL_CA_FALLBACK=ON',
            '-DCURL_BROTLI=OFF', '-DCURL_ZSTD=OFF', '-DCURL_USE_LIBSSH2=OFF', '-DCURL_USE_LIBPSL=OFF', '-DCURL_USE_GSSAPI=OFF',
            '-DUSE_NGHTTP2=OFF', '-DUSE_NGHTTP3=OFF', '-DUSE_NGTCP2=OFF', '-DUSE_LIBIDN2=OFF', '-DENABLE_ARES=OFF',
            '-DCURL_DISABLE_LDAP=ON',
            '-DBUILD_CURL_EXE=OFF', '-DBUILD_LIBCURL_DOCS=OFF', '-DBUILD_MISC_DOCS=OFF', '-DENABLE_CURL_MANUAL=OFF',
            '-DBUILD_TESTING=OFF', '-DBUILD_EXAMPLES=OFF',
        ]),
        // Its data in the library, nothing loaded at runtime
        'icu' => $build->icu($source, [
            '--with-data-packaging=static', '--enable-release', '--enable-icuio',
            '--disable-dyload', '--disable-extras', '--disable-tools', '--disable-tests', '--disable-samples',
        ]),
        default => throw new \LogicException(\sprintf('The library "%s" has no recipe in build_library().', $library)),
    };
}

/**
 * Extracts PHP, builds it and installs it into the buildroot: libphp.a, its
 * headers, php-config.
 */
function build_php(Build $build, string $archive): void
{
    $source = $build->extract($archive, 'php-src');
    $prefix = $build->path($build->buildroot);

    $options = [];
    foreach (explode(',', EXTENSIONS) as $extension) {
        foreach (CONFIGURE[$extension]['options'] as $option) {
            $options[] = str_replace('{prefix}', $prefix, $option);
        }
    }

    $build->configure($source, [
        "--prefix={$prefix}",
        '--disable-all', '--enable-embed=static', '--disable-cli', '--disable-cgi', '--disable-phpdbg',
        // No JIT: iOS runs no code it did not sign. Android would, but the
        // JIT of PCRE needs __clear_cache() from the runtime of clang, which
        // Rust does not link, and opcache's is off by default (opcache.jit)
        '--without-pcre-jit', '--disable-opcache-jit',
        ...array_values(array_unique($options)),
    ]);

    if ($build->target->isAndroid()) {
        adapt_to_bionic(variable('root_dir')."/{$source}/main/php_config.h");
    }

    $build->make($source);
    $build->make($source, ['install']);
}

/**
 * What configure finds in bionic, the C library of Android, but its headers
 * do not declare: PHP would not compile.
 */
function adapt_to_bionic(string $config): void
{
    $contents = (string) file_get_contents($config);

    // bionic has res_nsearch(), but declares neither it nor its state:
    // PHP queries DNS with res_search(), as on the systems without it
    $nsearch = "#define HAVE_RES_NSEARCH 1\n";
    // bionic has dn_skipname() as __dn_skipname(), as glibc does, but
    // declares neither; getdtablesize() is gone from its 64-bit headers
    $end = "#endif /* PHP_CONFIG_H */\n";
    $declarations = <<<'C'
        /* Nativella: what bionic has, but does not declare (.castor/static-php-mobile.php) */
        #ifndef __ASSEMBLER__
        int __dn_skipname(const unsigned char *, const unsigned char *);
        #endif
        #define dn_skipname __dn_skipname
        #define getdtablesize() ((int) sysconf(_SC_OPEN_MAX))


        C;

    foreach ([$nsearch, $end] as $expected) {
        if (!str_contains($contents, $expected)) {
            throw new \RuntimeException(\sprintf('%s has no "%s" any more: the configure script of PHP changed, check what bionic lacks again.', $config, trim($expected)));
        }
    }

    fs()->dumpFile($config, str_replace([$nsearch, $end], ["/* #undef HAVE_RES_NSEARCH */\n", $declarations.$end], $contents));
}

/**
 * The compilers of $target, and what identifies them in the stamp of the
 * build.
 */
function toolchain(Target $target): Toolchain
{
    if ($missing = missing_prerequisites($target)) {
        throw new \RuntimeException(\sprintf("PHP for %s builds with, on this machine (see docs/development.md#php-for-the-phones):\n - %s", $target->name, implode("\n - ", $missing)));
    }

    if ($target->isAndroid()) {
        if (!context()['run_on_host']) {
            // Nothing to do once built, until the Dockerfile changes
            \docker\build(service: 'android');
        }

        return Toolchain::android($target, ndk_home(), environment_variable($target, 'PATH'), build_triple($target));
    }

    return Toolchain::ios($target, build_triple($target));
}

/**
 * What the build of $target needs on this machine, and how to get it: nothing
 * for Android in Docker, which has it all.
 *
 * @return list<string>
 */
function missing_prerequisites(Target $target): array
{
    if ($target->isAndroid() && !context()['run_on_host']) {
        return [];
    }

    if (!$target->isAndroid() && !context()['macos']) {
        return ['macOS, with Xcode.'];
    }

    $missing = [];
    $finder = new ExecutableFinder();
    $commands = $target->isAndroid() ? ['c++', 'cc', 'cmake', 'make', 'perl', 'pkg-config'] : ['cmake', 'make', 'perl', 'pkg-config', 'xcrun'];
    foreach ($commands as $command) {
        if (null === $finder->find($command)) {
            $missing[] = "The {$command} command.";
        }
    }

    if ($target->isAndroid() && !getenv('NDK_HOME') && !glob(getenv('ANDROID_HOME').'/ndk/*', \GLOB_ONLYDIR)) {
        $missing[] = 'An NDK, in NDK_HOME or in ANDROID_HOME/ndk/ (sdkmanager "ndk;<version>").';
    }

    return $missing;
}

/**
 * What the commands of the build of $target see in $name.
 */
function environment_variable(Target $target, string $name): string
{
    if ($target->isAndroid() && !context()['run_on_host']) {
        return trim(docker_compose_run(['printenv', $name], context()->withQuiet(), service: 'android')->getOutput());
    }

    return (string) getenv($name);
}

/**
 * The machine that builds, as configure scripts name it: they cross compile
 * when the one they build for is another.
 */
function build_triple(Target $target): string
{
    if ($target->isAndroid() && !context()['run_on_host']) {
        return trim(docker_compose_run(['cc', '-dumpmachine'], context()->withQuiet(), service: 'android')->getOutput());
    }

    return trim(capture(['cc', '-dumpmachine']));
}

/**
 * A phone, or its simulator, as PHP is built for it.
 */
final readonly class Target
{
    private function __construct(
        /** "android-arm64": the directory of the build, in static-php/ */
        public string $name,
        /** "android" or "ios" */
        public string $platform,
        /** The platform as configure scripts name it: aarch64-linux-android */
        public string $host,
        /** The ABI of Android, or the SDK of Xcode */
        public string $abi,
        /** The target of OpenSSL's Configure */
        public string $openssl,
    ) {
    }

    public static function android(string $abi): self
    {
        $arch = match ($abi) {
            'arm64-v8a' => 'arm64',
            'x86_64' => 'x86_64',
            default => throw new \RuntimeException(\sprintf('Unsupported Android ABI "%s": arm64-v8a or x86_64.', $abi)),
        };

        return new self(
            name: "android-{$arch}",
            platform: 'android',
            host: ('arm64' === $arch ? 'aarch64' : $arch).'-linux-android',
            abi: $abi,
            openssl: "android-{$arch}",
        );
    }

    /**
     * configure scripts know iOS as Darwin: the compilers tell it from macOS.
     */
    public static function iphoneos(): self
    {
        return new self(
            name: 'ios-arm64',
            platform: 'ios',
            host: 'aarch64-apple-darwin',
            abi: 'iphoneos',
            openssl: 'ios64-xcrun',
        );
    }

    public static function iphonesimulator(): self
    {
        return new self(
            name: 'ios-simulator-arm64',
            platform: 'ios',
            host: 'aarch64-apple-darwin',
            abi: 'iphonesimulator',
            openssl: 'iossimulator-arm64-xcrun',
        );
    }

    public function isAndroid(): bool
    {
        return 'android' === $this->platform;
    }
}

/**
 * The compilers of a target, for the commands of its build.
 */
final readonly class Toolchain
{
    /**
     * @param array<string, string> $compilers          the compilers and their tools, for configure scripts
     * @param list<string>          $cmake              the options that make CMake build for the target
     * @param array<string, string> $opensslEnvironment what OpenSSL's Configure needs to find the compilers
     * @param list<string>          $opensslOptions     what OpenSSL's Configure needs for the target
     * @param list<string>          $strip              the command that strips the debug information of libraries
     */
    private function __construct(
        /** The NDK, or Xcode, and the version of the system: for the stamp of the build */
        public string $version,
        public array $compilers,
        public array $cmake,
        public array $opensslEnvironment,
        public array $opensslOptions,
        public array $strip,
        /** The machine that builds, for configure scripts */
        public string $build,
        /** "Linux" or "MacOSX": the platform of runConfigureICU, for the tools of ICU */
        public string $icuHost,
    ) {
    }

    /**
     * The clang of the NDK, for the API level of the template APK. OpenSSL
     * finds it in PATH, which is $path otherwise.
     */
    public static function android(Target $target, string $ndk, string $path, string $build): self
    {
        $api = android_api_level();
        // The NDK has its tools for x86_64 only, which macOS runs on arm64 too
        $bin = \sprintf('%s/toolchains/llvm/prebuilt/%s-x86_64/bin', $ndk, str_contains($build, 'darwin') ? 'darwin' : 'linux');

        return new self(
            version: 'ndk-'.basename($ndk)." android-{$api}",
            compilers: [
                'CC' => "{$bin}/{$target->host}{$api}-clang",
                'CXX' => "{$bin}/{$target->host}{$api}-clang++",
                'AR' => "{$bin}/llvm-ar",
                'RANLIB' => "{$bin}/llvm-ranlib",
                'NM' => "{$bin}/llvm-nm",
                'STRIP' => "{$bin}/llvm-strip",
            ],
            cmake: [
                "-DCMAKE_TOOLCHAIN_FILE={$ndk}/build/cmake/android.toolchain.cmake",
                "-DANDROID_ABI={$target->abi}",
                "-DANDROID_PLATFORM={$api}",
            ],
            opensslEnvironment: [
                'ANDROID_NDK_ROOT' => $ndk,
                'PATH' => "{$bin}:{$path}",
            ],
            // OpenSSL reads the API level there, and passes it on top of the
            // compiler of the NDK, which defines it already: the same
            opensslOptions: ["-D__ANDROID_API__={$api}", '-Wno-macro-redefined'],
            strip: ["{$bin}/llvm-strip", '--strip-debug'],
            build: $build,
            icuHost: str_contains($build, 'darwin') ? 'MacOSX' : 'Linux',
        );
    }

    /**
     * The clang of Xcode, for the SDK of $target and IOS_VERSION. OpenSSL
     * takes it from the environment too.
     */
    public static function ios(Target $target, string $build): self
    {
        $find = static fn (string $tool): string => trim(capture(['xcrun', '--sdk', $target->abi, '--find', $tool]));
        $sdk = trim(capture(['xcrun', '--sdk', $target->abi, '--show-sdk-path']));
        $clang = $find('clang');
        $clangxx = $find('clang++');
        $flags = '-target arm64-apple-ios'.IOS_VERSION.('iphonesimulator' === $target->abi ? '-simulator' : '')." -isysroot {$sdk}";
        $xcode = preg_replace('/\s+/', ' ', trim(capture(['xcodebuild', '-version'])));

        return new self(
            version: "{$xcode} {$target->abi} ios-".IOS_VERSION,
            compilers: [
                'CC' => "{$clang} {$flags}",
                'CXX' => "{$clangxx} {$flags}",
                'AR' => $find('ar'),
                'RANLIB' => $find('ranlib'),
                'NM' => $find('nm'),
                'STRIP' => $find('strip'),
            ],
            cmake: [
                '-DCMAKE_SYSTEM_NAME=iOS',
                "-DCMAKE_OSX_SYSROOT={$target->abi}",
                '-DCMAKE_OSX_ARCHITECTURES=arm64',
                '-DCMAKE_OSX_DEPLOYMENT_TARGET='.IOS_VERSION,
            ],
            opensslEnvironment: [],
            opensslOptions: [],
            strip: [$find('strip'), '-S'],
            build: $build,
            icuHost: 'MacOSX',
        );
    }
}

/**
 * The build of PHP for a target, under way: where it is, and how its commands
 * run.
 */
final readonly class Build
{
    /**
     * For size, as the applications keep only what PHP uses, and position
     * independent, as they are libraries.
     */
    private const string FLAGS = '-Os -fPIC -ffunction-sections -fdata-sections';

    public function __construct(
        public Target $target,
        public Toolchain $toolchain,
        /** From the root of the repository */
        public string $buildroot,
        /** Where the sources are extracted, from the root of the repository */
        public string $source,
    ) {
    }

    /**
     * Where $path, from the root of the repository, is for the commands of
     * the build: in the android service, or on this machine.
     */
    public function path(string $path): string
    {
        return $this->onHost() ? variable('root_dir')."/{$path}" : builder_path($path);
    }

    /**
     * Runs a command of the build in $dir, from the root of the repository,
     * and throws when it fails.
     *
     * @param list<string>          $command
     * @param array<string, string> $environment
     */
    public function run(array $command, string $dir, array $environment = []): void
    {
        if (!$this->onHost()) {
            run_in_builder($command, $this->path($dir), $environment, 'android');

            return;
        }

        run($command, context: context()->withWorkingDirectory(variable('root_dir')."/{$dir}")->withEnvironment($environment));
    }

    /**
     * Extracts $archive, from the root of the repository, into a directory of
     * its own.
     *
     * @return string the directory, from the root of the repository
     */
    public function extract(string $archive, string $name): string
    {
        $dir = "{$this->source}/{$name}";
        fs()->mkdir(variable('root_dir')."/{$dir}");
        $this->run(['tar', 'xf', $this->path($archive), '--strip-components=1'], $dir);

        return $dir;
    }

    /**
     * A library built with autoconf: static only, position independent.
     *
     * @param list<string> $options
     * @param list<string> $targets        what make builds and installs
     * @param list<string> $subdirectories built and installed the same way, after $dir
     */
    public function autoconf(string $dir, array $options = [], array $targets = ['install'], array $subdirectories = []): void
    {
        $this->configure($dir, ['--prefix='.$this->path($this->buildroot), '--enable-static', '--disable-shared', '--with-pic', ...$options]);

        foreach (['', ...$subdirectories] as $subdirectory) {
            $this->make(rtrim("{$dir}/{$subdirectory}", '/'), $targets);
        }
    }

    /**
     * The configure script of autoconf, cross compiling.
     *
     * @param list<string> $options
     */
    public function configure(string $dir, array $options): void
    {
        $this->run(['./configure', '--quiet', "--host={$this->target->host}", "--build={$this->toolchain->build}", ...$options], $dir, $this->environment());
    }

    /**
     * @param list<string> $targets
     */
    public function make(string $dir, array $targets = []): void
    {
        $this->run(['make', '-s', '-j'.cpu_count(), ...$targets], $dir, $this->environment());
    }

    /**
     * zlib, with its own configure script: it takes the compilers from the
     * environment, and the platform from them.
     */
    public function zlib(string $dir): void
    {
        $this->run(['./configure', '--static', '--prefix='.$this->path($this->buildroot)], $dir, $this->environment());
        $this->make($dir, ['install']);
    }

    /**
     * OpenSSL, with its own Configure, and its own targets for the phones.
     *
     * @param list<string> $options
     */
    public function openssl(string $dir, array $options): void
    {
        $environment = $this->environment($this->toolchain->opensslEnvironment);

        $this->run(['./Configure', $this->target->openssl, ...$this->toolchain->opensslOptions, ...$options], $dir, $environment);
        $this->run(['make', '-s', '-j'.cpu_count()], $dir, $environment);
        $this->run(['make', '-s', 'install_sw'], $dir, $environment);
    }

    /**
     * A library built with CMake, for the target: static only, position
     * independent. CMake takes the compilers from its own options.
     *
     * @param list<string> $options
     */
    public function cmake(string $dir, array $options): void
    {
        $prefix = $this->path($this->buildroot);
        $environment = array_diff_key($this->environment(), $this->toolchain->compilers);

        $this->run([
            'cmake', '-S', '.', '-B', 'build', '-Wno-dev', '--log-level=WARNING',
            ...$this->toolchain->cmake,
            '-DCMAKE_BUILD_TYPE=Release',
            // Errors and warnings only, as make -s
            '-DCMAKE_RULE_MESSAGES=OFF', '-DCMAKE_INSTALL_MESSAGE=NEVER',
            "-DCMAKE_INSTALL_PREFIX={$prefix}",
            '-DCMAKE_INSTALL_LIBDIR=lib',
            // The libraries built before, not the ones of the machine
            "-DCMAKE_FIND_ROOT_PATH={$prefix}",
            '-DCMAKE_POSITION_INDEPENDENT_CODE=ON',
            '-DBUILD_SHARED_LIBS=OFF',
            ...$options,
        ], $dir, $environment);
        $this->run(['cmake', '--build', 'build', '-j', (string) cpu_count()], $dir, $environment);
        $this->run(['cmake', '--install', 'build'], $dir, $environment);
    }

    /**
     * ICU: its tools for the machine that builds first, which the build for
     * the target runs to package its data, then ICU for the target, with the
     * definitions of static-php-cli.
     *
     * @param list<string> $options
     */
    public function icu(string $dir, array $options): void
    {
        $tools = "{$dir}/tools";
        $target = "{$dir}/target";
        fs()->mkdir([variable('root_dir')."/{$tools}", variable('root_dir')."/{$target}"]);

        // With the compilers of the machine
        $this->run(['../source/runConfigureICU', $this->toolchain->icuHost, '--disable-shared', '--enable-static', '--disable-tests', '--disable-samples', '--disable-extras'], $tools);
        $this->run(['make', '-s', '-j'.cpu_count()], $tools);

        $environment = $this->environment([
            'CPPFLAGS' => '-DU_CHARSET_IS_UTF8=1 -DU_USING_ICU_NAMESPACE=1 -DU_STATIC_IMPLEMENTATION=1',
            'CXXFLAGS' => self::FLAGS.' -std=c++17',
        ]);
        $this->run([
            '../source/configure', '--quiet', "--host={$this->target->host}", "--build={$this->toolchain->build}",
            '--with-cross-build='.$this->path($tools),
            '--prefix='.$this->path($this->buildroot), '--enable-static', '--disable-shared',
            ...$options,
        ], $target, $environment);
        $this->run(['make', '-s', '-j'.cpu_count()], $target, $environment);
        $this->run(['make', '-s', 'install'], $target, $environment);
    }

    /**
     * The compilers of the target, and the libraries built before, from the
     * buildroot only, not from the machine.
     *
     * @param array<string, string> $overrides
     *
     * @return array<string, string>
     */
    private function environment(array $overrides = []): array
    {
        $prefix = $this->path($this->buildroot);

        return [
            ...$this->toolchain->compilers,
            'CFLAGS' => self::FLAGS,
            'CXXFLAGS' => self::FLAGS,
            'CPPFLAGS' => "-I{$prefix}/include",
            'LDFLAGS' => "-L{$prefix}/lib",
            'PKG_CONFIG_LIBDIR' => "{$prefix}/lib/pkgconfig",
            'PKG_CONFIG_PATH' => '',
            ...$overrides,
        ];
    }

    /**
     * iOS builds on the Mac, whatever the context; Android in the android
     * service, or on the host in the host context.
     */
    private function onHost(): bool
    {
        return !$this->target->isAndroid() || context()['run_on_host'];
    }
}

/**
 * The API level the template APK runs from: the minSdkVersion of
 * crates/nativella/tauri.android.conf.json. PHP and its libraries are built
 * for it.
 */
function android_api_level(): int
{
    $config = json_decode((string) file_get_contents(variable('root_dir').'/crates/nativella/tauri.android.conf.json'), true, flags: \JSON_THROW_ON_ERROR);

    return $config['bundle']['android']['minSdkVersion'] ?? throw new \RuntimeException('crates/nativella/tauri.android.conf.json has no bundle.android.minSdkVersion.');
}

/**
 * The processors of this machine, for the jobs of make.
 */
function cpu_count(): int
{
    return max(1, (int) capture(['getconf', '_NPROCESSORS_ONLN']));
}
