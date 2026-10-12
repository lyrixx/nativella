<?php

namespace mobile;

use Castor\Attribute\AsArgument;
use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use project\Project;
use Symfony\Component\Process\ExecutableFinder;

use function Castor\capture;
use function Castor\context;
use function Castor\finder;
use function Castor\fs;
use function Castor\http_download;
use function Castor\http_request;
use function Castor\io;
use function Castor\run;
use function Castor\variable;
use function docker\builder_path;
use function docker\docker_compose_run;
use function docker\run_in_builder;
use function nativella\bundle;
use function project\demo;

/** The release of NativePHP's builds of PHP for phones: see fetch_php() */
const NATIVEPHP_RELEASE = '4.0.0';

const PHP_BRANCH = '8.5';

/**
 * In the android service, the builder with the Android SDK, which this task
 * builds first; or on the host, with its SDK, JDK and tools, in the host
 * context. The demo is staged for production and bundled in the builder, then
 * compiled into the application (see docs/mobile.md).
 */
#[AsTask(description: 'Builds an Android APK (arm64) of a demo')]
function android(
    #[AsArgument(description: 'The demo to build, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'Build a release APK, still to be signed, instead of a debug one')]
    bool $release = false,
): void {
    $project = demo($demo);
    $onHost = context()['run_on_host'];

    if ($onHost && $missing = missing_android_prerequisites()) {
        throw new \RuntimeException(\sprintf("The Android build needs, on the host (see docs/mobile.md):\n - %s", implode("\n - ", $missing)));
    }

    if (!$onHost) {
        // Nothing to do once built, until the Dockerfile changes
        \docker\build(service: 'android');
    }

    io()->title("Building an APK of {$project->path}");

    $bundle = bundle($project);
    $php = fetch_php('android');
    $ndk = ndk_home();
    $service = $onHost ? null : 'android';
    $workDir = builder_path('crates/nativella');

    $environment = [
        'NATIVELLA_BUNDLE' => builder_path($bundle),
        // PHP for Android: libphp.a and its headers, which the stand-ins of
        // mobile/php describe to ext-php-rs
        'NATIVELLA_PHP_DIR' => builder_path($php),
        'NATIVELLA_PHP_LIB_DIR' => builder_path("{$php}/staticLibs/arm64-v8a"),
        'NDK_HOME' => $ndk,
        'PHP' => builder_path('mobile/php/php'),
        'PHP_CONFIG' => builder_path('mobile/php/php-config'),
        // PHP's headers use GNU extensions of bionic (mempcpy), as PHP itself
        // was built. bindgen targets the API level 24, the first with all of
        // them.
        'BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android' => "--target=aarch64-linux-android24 --sysroot={$ndk}/toolchains/llvm/prebuilt/linux-x86_64/sysroot -D_GNU_SOURCE",
        'CFLAGS_aarch64_linux_android' => '-D_GNU_SOURCE',
    ];

    $config = tauri_config($project);
    // The Android project names its package after the identifier, and holds
    // the path of the SDK
    $generation = $config['identifier'].' '.($onHost ? getenv('ANDROID_HOME') : 'docker');
    if (generated('android') !== $generation) {
        fs()->remove(variable('root_dir').'/crates/nativella/gen/android');
        run_in_builder(['cargo', 'tauri', 'android', 'init', '--ci', '--config', json_encode($config, \JSON_THROW_ON_ERROR)], $workDir, $environment, $service);
        fs()->dumpFile(variable('root_dir').'/crates/nativella/gen/android/.nativella-generation', $generation);
    }

    run_in_builder([
        'cargo', 'tauri', 'android', 'build', '--ci', '--target', 'aarch64', '--apk',
        ...($release ? [] : ['--debug']),
        '--config', json_encode($config, \JSON_THROW_ON_ERROR),
        ...features($project),
    ], $workDir, $environment, $service);

    io()->success('The APK: '.built(variable('root_dir').'/crates/nativella/gen/android/app/build/outputs/apk', '*.apk', $bundle));
}

/**
 * On the Mac, with Xcode, XcodeGen, the aarch64-apple-ios-sim target of Rust
 * and the Tauri command. A simulator build needs no signing.
 *
 * EXPERIMENTAL: .github/workflows/mobile.yml runs it on a macOS runner.
 */
#[AsTask(description: 'Builds an iOS simulator app (arm64) of a demo, on macOS (experimental)')]
function ios(
    #[AsArgument(description: 'The demo to build, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'Build a release app instead of a debug one')]
    bool $release = false,
    #[AsOption(description: 'A bundle of the demo made beforehand (nativella bundle), from the root of the repository: by default, the demo is staged and bundled in the builder')]
    ?string $bundle = null,
): void {
    if (!context()['macos']) {
        throw new \RuntimeException('An iOS build needs macOS, with Xcode.');
    }

    $project = demo($demo);
    $root = variable('root_dir');

    io()->title("Building an iOS simulator app of {$project->path}");

    $bundle ??= bundle($project);
    $php = fetch_php('ios');

    $environment = [
        'NATIVELLA_BUNDLE' => "{$root}/{$bundle}",
        // PHP for iOS: libphp.a and the libraries it needs for the simulator,
        // its headers, and the stand-ins describing it to ext-php-rs
        'NATIVELLA_PHP_DIR' => "{$root}/{$php}",
        'NATIVELLA_PHP_LIB_DIR' => "{$root}/{$php}/Libraries/iphonesimulator",
        'PHP' => "{$root}/mobile/php/php",
        'PHP_CONFIG' => "{$root}/mobile/php/php-config",
        // bindgen parses PHP's headers for the simulator, with its SDK, and
        // for the iOS version Tauri targets by default
        // (bundle.iOS.minimumSystemVersion): without one, clang takes an iOS
        // without thread-local storage, which PHP uses
        'BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_ios_sim' => '--target=arm64-apple-ios15.0-simulator -isysroot '.trim(capture(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-path'])),
    ];

    // iOS asks the user for a permission only with a reason, from
    // Info.plist: Tauri merges this file into it
    fs()->dumpFile("{$root}/crates/nativella/Info.ios.plist", info_plist($project));

    $c = context()->withEnvironment($environment)->withWorkingDirectory("{$root}/crates/nativella");
    $config = tauri_config($project);

    // The Xcode project is named after the application
    if (generated('apple') !== $config['identifier']) {
        fs()->remove("{$root}/crates/nativella/gen/apple");
        run(['cargo', 'tauri', 'ios', 'init', '--ci', '--config', json_encode($config, \JSON_THROW_ON_ERROR)], context: $c);
        fs()->dumpFile("{$root}/crates/nativella/gen/apple/.nativella-generation", $config['identifier']);
    }

    // Tauri hands the Rust build to Xcode, which runs it from crates/nativella
    // with TAURI_*, CARGO_*, RUST_* and PATH only: cargo sets the rest from its
    // configuration, there and for the time of the build
    $cargoConfig = "{$root}/crates/nativella/.cargo/config.toml";
    fs()->dumpFile($cargoConfig, cargo_env($environment));

    try {
        run([
            'cargo', 'tauri', 'ios', 'build', '--ci', '--target', 'aarch64-sim', '--no-sign',
            ...($release ? [] : ['--debug']),
            '--config', json_encode($config, \JSON_THROW_ON_ERROR),
            ...features($project),
        ], context: $c);
    } finally {
        fs()->remove(\dirname($cargoConfig));
    }

    io()->success('The app: '.built("{$root}/crates/nativella/gen/apple/build", '*.app', $bundle));
}

/**
 * Downloads a static PHP build for Android or iOS: libphp.a, the libraries
 * it needs and its headers, into mobile/php/dist/<platform>-<php version>/.
 *
 * They come from NativePHP's public builds (https://github.com/NativePHP,
 * MIT, built with static-php-cli and the NDK / Xcode) until Nativella builds
 * its own.
 *
 * @return string the directory, from the root of the repository
 */
function fetch_php(string $platform): string
{
    $manifest = http_request('GET', 'https://bin.nativephp.com/main/'.NATIVEPHP_RELEASE.'.json')->toArray();
    $urls = array_filter(
        $manifest['versions'][PHP_BRANCH][$platform] ?? [],
        static fn (string $url): bool => !str_ends_with($url, '-icu.zip'),
    );
    $url = reset($urls) ?: throw new \RuntimeException(\sprintf('NativePHP has no PHP %s build for %s in its release %s.', PHP_BRANCH, $platform, NATIVEPHP_RELEASE));

    $dir = "mobile/php/dist/{$platform}-".preg_replace('/.*-php/', '', basename($url, '.zip'));
    $root = variable('root_dir');

    if (!is_dir("{$root}/{$dir}")) {
        http_download($url, "{$root}/{$dir}.zip");
        run_in_builder(['unzip', '-q', "{$dir}.zip", '-d', $dir]);
        fs()->remove("{$root}/{$dir}.zip");
    }

    return $dir;
}

/**
 * @return list<string> what is missing on the host, and how to get it
 */
function missing_android_prerequisites(): array
{
    $missing = [];
    $finder = new ExecutableFinder();

    $androidHome = getenv('ANDROID_HOME') ?: null;
    if (null === $androidHome || !is_dir($androidHome)) {
        $missing[] = 'ANDROID_HOME, pointing to the Android SDK.';
    } elseif (!getenv('NDK_HOME') && !glob("{$androidHome}/ndk/*", \GLOB_ONLYDIR)) {
        $missing[] = \sprintf('An NDK in %s/ndk/ (sdkmanager "ndk;<version>"), or NDK_HOME.', $androidHome);
    }

    $javaHome = getenv('JAVA_HOME') ?: null;
    $javaVersion = null === $javaHome ? null : java_major_version($javaHome);
    if (null === $javaVersion || $javaVersion < 17) {
        $missing[] = \sprintf('JAVA_HOME, pointing to a JDK 17 or later%s.', null === $javaHome ? '' : " ({$javaHome} is not one)");
    }

    foreach (['cargo', 'composer', 'php', 'rustup'] as $command) {
        if (null === $finder->find($command)) {
            $missing[] = "The {$command} command.";
        }
    }

    if (null !== $finder->find('rustup') && !\in_array('aarch64-linux-android', explode("\n", capture(['rustup', 'target', 'list', '--installed'])), true)) {
        $missing[] = 'The aarch64-linux-android Rust target: rustup target add aarch64-linux-android.';
    }

    if (null === $finder->find('cargo-tauri')) {
        $missing[] = "The Tauri command: cargo install tauri-cli --version '^2' --locked.";
    }

    return $missing;
}

/**
 * The NDK of the Android build: the one of the android service, or on the
 * host, NDK_HOME or the most recent NDK of the SDK.
 */
function ndk_home(): string
{
    if (!context()['run_on_host']) {
        return trim(docker_compose_run(['printenv', 'NDK_HOME'], context()->withQuiet(), service: 'android')->getOutput());
    }

    if ($ndk = getenv('NDK_HOME')) {
        return $ndk;
    }

    $ndks = glob(getenv('ANDROID_HOME').'/ndk/*', \GLOB_ONLYDIR) ?: [];
    usort($ndks, static fn (string $a, string $b): int => version_compare(basename($a), basename($b)));

    return end($ndks) ?: throw new \LogicException('missing_android_prerequisites() checks that there is an NDK.');
}

/**
 * From the release file of the JDK: JAVA_VERSION="21.0.12.1".
 */
function java_major_version(string $javaHome): ?int
{
    if (!is_file("{$javaHome}/release")) {
        return null;
    }

    $release = (string) file_get_contents("{$javaHome}/release");

    if (!preg_match('{^JAVA_VERSION="(?:1\.)?(\d+)}m', $release, $matches)) {
        return null;
    }

    return (int) $matches[1];
}

/**
 * The identity of the application, from the extra.nativella section of its
 * composer.json: it overrides the placeholder of tauri.conf.json.
 *
 * @return array{identifier: string, productName?: string, version?: string}
 */
function tauri_config(Project $project): array
{
    $nativella = nativella_config($project);

    $identifier = $nativella['identifier'] ?? throw new \RuntimeException(\sprintf('%s has no extra.nativella.identifier in its composer.json.', $project->path));

    return ['identifier' => $identifier] + array_filter([
        'productName' => $nativella['name'] ?? null,
        'version' => $nativella['version'] ?? null,
    ]);
}

/**
 * The APIs of phones the application asks for, and only them: each one is a
 * feature of the nativella crate, which brings its permissions.
 *
 * @return list<string> the options of `cargo tauri … build`
 */
function features(Project $project): array
{
    $permissions = nativella_config($project)['permissions'] ?? [];

    return [] === $permissions ? [] : ['--features', implode(',', $permissions)];
}

/**
 * The reasons iOS gives the user when the application asks for a permission.
 */
function info_plist(Project $project): string
{
    $nativella = nativella_config($project);
    $name = $nativella['name'] ?? $project->name();
    $reasons = [
        'biometrics' => ['NSFaceIDUsageDescription' => "{$name} uses Face ID to check that it is you."],
        'geolocation' => ['NSLocationWhenInUseUsageDescription' => "{$name} uses the location of the phone."],
    ];

    $keys = '';
    foreach ($nativella['permissions'] ?? [] as $permission) {
        foreach ($reasons[$permission] ?? [] as $key => $reason) {
            $keys .= "    <key>{$key}</key>\n    <string>".htmlspecialchars($reason, \ENT_XML1)."</string>\n";
        }
    }

    return <<<XML
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
        {$keys}</dict>
        </plist>

        XML;
}

/**
 * @return array{identifier?: string, name?: string, version?: string, permissions?: list<string>}
 */
function nativella_config(Project $project): array
{
    $composer = json_decode((string) file_get_contents($project->directory().'/composer.json'), true, flags: \JSON_THROW_ON_ERROR);

    return $composer['extra']['nativella'] ?? [];
}

/**
 * The [env] section of a cargo configuration: the variables cargo gives the
 * build scripts and the compiler.
 *
 * @param array<string, string> $environment
 */
function cargo_env(array $environment): string
{
    $toml = "[env]\n";
    foreach ($environment as $name => $value) {
        // A JSON string without escaped slashes is a TOML basic string
        $toml .= $name.' = '.json_encode($value, \JSON_THROW_ON_ERROR | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE)."\n";
    }

    return $toml;
}

/**
 * What the Android or Xcode project of crates/nativella/gen/ was generated
 * for, null when there is none.
 */
function generated(string $platform): ?string
{
    $file = variable('root_dir')."/crates/nativella/gen/{$platform}/.nativella-generation";

    return is_file($file) ? (string) file_get_contents($file) : null;
}

/**
 * The application the build has just written: newer than the bundle.
 */
function built(string $dir, string $name, string $bundle): string
{
    $since = (int) filemtime(variable('root_dir')."/{$bundle}");

    foreach (finder()->in($dir)->name($name)->filter(static fn (\SplFileInfo $file): bool => $file->getMTime() >= $since) as $file) {
        return substr($file->getPathname(), \strlen(variable('root_dir')) + 1);
    }

    throw new \RuntimeException(\sprintf('The build wrote no %s in %s.', $name, $dir));
}
