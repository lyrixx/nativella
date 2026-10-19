<?php

namespace mobile;

use Castor\Attribute\AsArgument;
use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use project\Project;
use static_php\mobile\Target as PhpTarget;
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
use function nativella\stage;
use function project\demo;
use function static_php\mobile\android_api_level;
use function static_php\mobile\build as php_build;

/** The release of NativePHP's builds of PHP for phones: see fetch_php() */
const NATIVEPHP_RELEASE = '4.0.0';

const PHP_BRANCH = '8.5';

/** What the runtime writes when PHP answers a page: "[php] GET <url> 200 <time> ms" */
const PHP_ANSWERED = '/\[php\] GET \S+ 200 /';

/**
 * What a phone application may ask for, each one a feature of the nativella
 * crate: the template APK has them all. Keep in sync with the features of
 * crates/nativella/Cargo.toml.
 */
const PHONE_APIS = ['biometrics', 'geolocation', 'haptics'];

/**
 * The template APK first (mobile:android-template), then `nativella
 * android`, in the builder, makes the APK of the demo from it, staged for
 * production (see docs/mobile.md).
 */
#[AsTask(description: 'Builds an Android APK of a demo, from the template APK')]
function android(
    #[AsArgument(description: 'The demo to build, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'Make it from a release template, smaller, instead of a debug one')]
    bool $release = false,
    #[AsOption(description: 'The ABI: arm64-v8a, for phones, or x86_64, for the emulators of x86_64 computers')]
    string $abi = 'arm64-v8a',
): void {
    $project = demo($demo);
    $template = android_template($release, abi: $abi);

    io()->title("Making the APK of {$project->path}");

    $apk = android_apk($demo, $abi);
    run_in_builder(['cargo', 'run', '-q', '-p', 'nativella', '--', 'android', stage($project), '--template', builder_path($template), '-o', builder_path($apk)]);

    io()->success("The APK: {$apk}");
}

/**
 * The application library, PHP linked in, and the Java side of Tauri and of
 * its plugins, with every phone API, for the package of
 * crates/nativella/tauri.conf.json: what `nativella android` makes an
 * application of, with a PHP project. In the android service, the builder
 * with the Android SDK, which this task builds first; or on the host, with
 * its SDK, JDK and tools, in the host context.
 *
 * @return string the template, from the root of the repository
 */
#[AsTask(name: 'android-template', description: 'Builds the template APK that "nativella android" makes applications of')]
function android_template(
    #[AsOption(description: 'Build a release template, without debug symbols, instead of a debug one')]
    bool $release = false,
    #[AsOption(description: 'Where to copy it, from the root of the repository, with its third-party notices and their checksums next to it')]
    ?string $output = null,
    #[AsOption(description: 'The ABI: arm64-v8a, for phones, or x86_64, for the emulators of x86_64 computers')]
    string $abi = 'arm64-v8a',
): string {
    $onHost = context()['run_on_host'];

    if ($onHost && $missing = missing_android_prerequisites()) {
        throw new \RuntimeException(\sprintf("The Android build needs, on the host (see docs/mobile.md):\n - %s", implode("\n - ", $missing)));
    }

    // PHP for Android first, in the android service, which it builds
    $target = PhpTarget::android($abi);
    $php = php_build($target);

    io()->title("Building the template APK ({$abi})");

    $ndk = ndk_home();
    $service = $onHost ? null : 'android';
    $workDir = builder_path('crates/nativella');
    $root = variable('root_dir');
    // The target of Rust, for the variables of its build, and its first
    // part for the Tauri command
    $rustTarget = $target->host;
    $variable = str_replace('-', '_', $rustTarget);

    $environment = [
        // PHP for Android: libphp.a, the libraries it needs and its headers,
        // which the stand-in of mobile/php and php-config describe to
        // ext-php-rs
        'NATIVELLA_PHP_DIR' => builder_path($php),
        'NATIVELLA_PHP_LIB_DIR' => builder_path("{$php}/lib"),
        'NDK_HOME' => $ndk,
        'PHP' => builder_path('mobile/php/php'),
        'PHP_CONFIG' => builder_path("{$php}/bin/php-config"),
        // PHP's headers use GNU extensions of bionic (mempcpy), as PHP itself
        // was built, for the API level of the template
        "BINDGEN_EXTRA_CLANG_ARGS_{$variable}" => \sprintf('--target=%s%d --sysroot=%s/toolchains/llvm/prebuilt/%s-x86_64/sysroot -D_GNU_SOURCE', $rustTarget, android_api_level(), $ndk, $onHost && context()['macos'] ? 'darwin' : 'linux'),
        "CFLAGS_{$variable}" => '-D_GNU_SOURCE',
    ];

    // The Android project names its package, and so the JNI functions of the
    // library, after the identifier of tauri.conf.json, and holds the path of
    // the SDK
    $identifier = json_decode((string) file_get_contents("{$root}/crates/nativella/tauri.conf.json"), true, flags: \JSON_THROW_ON_ERROR)['identifier'];
    $generation = $identifier.' '.($onHost ? getenv('ANDROID_HOME') : 'docker');
    if (generated('android') !== $generation) {
        fs()->remove("{$root}/crates/nativella/gen/android");
        run_in_builder(['cargo', 'tauri', 'android', 'init', '--ci'], $workDir, $environment, $service);
        fs()->dumpFile("{$root}/crates/nativella/gen/android/.nativella-generation", $generation);
    }

    $variant = $release ? 'release' : 'debug';
    $build = static fn () => run_in_builder([
        'cargo', 'tauri', 'android', 'build', '--ci', '--target', explode('-', $rustTarget)[0], '--apk',
        ...($release ? [] : ['--debug']),
        '--features', implode(',', PHONE_APIS),
    ], $workDir, $environment, $service);
    $build();

    // The third-party notices, an asset of the template, and so of every
    // application made of it. Gradle lists the Java libraries once the build
    // of the library generated the settings of the plugins: when the
    // notices change, Gradle packages the template again, the library being
    // built already.
    $notices = \notices\android($target, $php, PHONE_APIS, \notices\gradle_libraries('crates/nativella/gen/android', 'universal'.ucfirst($variant).'RuntimeClasspath', $environment, $service));
    $asset = "{$root}/crates/nativella/gen/android/app/src/main/assets/".\notices\FILE;
    if (!is_file($asset) || file_get_contents($asset) !== file_get_contents("{$root}/{$notices}")) {
        fs()->copy("{$root}/{$notices}", $asset, overwriteNewerFiles: true);
        $build();
    }

    // Gradle leaves it as it is when nothing changed. Unsigned in release:
    // `nativella android` signs the applications.
    $dir = "{$root}/crates/nativella/gen/android/app/build/outputs/apk/universal/{$variant}";
    $apks = iterator_to_array(finder()->in($dir)->files()->name('*.apk')->sortByModifiedTime(), false);
    $built = end($apks) ?: throw new \RuntimeException(\sprintf('The build wrote no APK in %s.', $dir));
    $template = $output ?? 'target/nativella/android-template-'.$variant.abi_suffix($abi).'.apk';
    fs()->copy($built->getPathname(), "{$root}/{$template}", overwriteNewerFiles: true);
    // As the release publishes it, with the files of the other platforms
    if (null !== $output) {
        $dir = \dirname("{$root}/{$output}");
        fs()->copy("{$root}/{$notices}", "{$dir}/THIRD_PARTY_NOTICES-android.txt", overwriteNewerFiles: true);
        $checksums = '';
        foreach ([basename($output), 'THIRD_PARTY_NOTICES-android.txt'] as $file) {
            $checksums .= hash_file('sha256', "{$dir}/{$file}")."  {$file}\n";
        }
        fs()->dumpFile("{$dir}/SHA256SUMS-android", $checksums);
    }

    io()->success("The template: {$template}");

    return $template;
}

/**
 * Installs the APK mobile:android built, afresh, on the emulator, or the
 * phone, that adb sees, opens it, and waits for PHP to answer a page. What
 * the app writes to logcat goes next to the screenshot. On the host, with the
 * adb of ANDROID_HOME, or of PATH; ANDROID_SERIAL picks a device when adb
 * sees several.
 *
 * .github/workflows/mobile.yml runs it in an emulator.
 */
#[AsTask(name: 'android-emulator', description: 'Opens the APK of a demo on an emulator, or a phone, waits for PHP to answer a page, and takes a screenshot')]
function android_emulator(
    #[AsArgument(description: 'The demo to open, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'The screenshot, from the root of the repository')]
    string $screenshot = 'target/nativella/android-emulator.png',
    #[AsOption(description: 'The time PHP has to answer, in seconds')]
    int $wait = 120,
    #[AsOption(description: 'An APK of the demo built beforehand, from the root of the repository: by default, the last one "castor mobile:android" built')]
    ?string $apk = null,
    #[AsOption(description: 'The ABI of the APK "castor mobile:android" built: arm64-v8a, or x86_64')]
    string $abi = 'arm64-v8a',
): void {
    $project = demo($demo);
    // The application ID of the APK: the identifier, its dashes as
    // underscores. Keep in sync with application_id() in
    // crates/nativella/src/apk/mod.rs
    $applicationId = str_replace('-', '_', tauri_config($project)['identifier']);
    $root = variable('root_dir');

    $apk = "{$root}/".($apk ?? android_apk($demo, $abi));
    if (!is_file($apk)) {
        throw new \RuntimeException(\sprintf('There is no APK in %s: run "castor mobile:android %s --abi=%s" first.', $apk, $demo, $abi));
    }

    $adb = adb();
    $device = run([$adb, 'get-serialno'], context: context()->withQuiet()->withAllowFailure());
    if (!$device->isSuccessful()) {
        throw new \RuntimeException(\sprintf("adb sees no device: start an emulator, or plug a phone in, or set ANDROID_SERIAL if it sees several (adb devices).\n%s", trim($device->getErrorOutput())));
    }

    io()->title(\sprintf('Opening %s on %s', $project->path, trim($device->getOutput())));

    // Until the first unlock after a boot, the storage of the user stays
    // encrypted, and Android starts no application
    if ('true' !== capture([$adb, 'shell', 'getprop', 'sys.user.0.ce_available'])) {
        throw new \RuntimeException('The device is locked: unlock it, then run the task again.');
    }

    // Those of a previous run would pass for this one's
    fs()->remove(["{$root}/{$screenshot}", "{$root}/".log_file($screenshot)]);

    // Afresh, as on a new phone: a previous install may be signed with another
    // debug key, and its data would hide a first launch
    run([$adb, 'uninstall', $applicationId], context: context()->withQuiet()->withAllowFailure());
    run([$adb, 'install', $apk]);
    run([$adb, 'logcat', '-c']);
    // The activity of the launcher: its class is in the package of the
    // template, not of the application
    $resolved = explode("\n", trim(capture([$adb, 'shell', 'cmd', 'package', 'resolve-activity', '--brief', '-c', 'android.intent.category.LAUNCHER', $applicationId])));
    run([$adb, 'shell', 'am', 'start', '-W', '-n', end($resolved)]);

    $output = static fn (): string => capture([$adb, 'logcat', '-d', '-s', 'nativella']);
    $running = static fn (): bool => '' !== trim(run([$adb, 'shell', 'pidof', $applicationId], context: context()->withQuiet()->withAllowFailure())->getOutput());

    io()->comment("Waiting up to {$wait} seconds for PHP to answer a page...");
    $log = wait_for_a_page($wait, $output, $running);

    // Even if the app is gone: the screenshot shows what is left
    fs()->mkdir(\dirname("{$root}/{$screenshot}"));
    run([$adb, 'shell', 'screencap', '-p', '/data/local/tmp/nativella-screenshot.png']);
    run([$adb, 'pull', '/data/local/tmp/nativella-screenshot.png', "{$root}/{$screenshot}"], context: context()->withQuiet());

    check_a_page_answered($screenshot, $log, $running(), $wait);
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
        'NATIVELLA_PHP_LIB_DIR' => $root.'/'.simulator_libraries($php),
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
        link_resolver("{$root}/crates/nativella/gen/apple");
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
 * Boots the simulator mobile:ios-simulator opens the app in, and waits for
 * it: a few minutes on the macOS runners of GitHub, which the Mobile
 * workflow spends building the app, running it in the background.
 */
#[AsTask(name: 'ios-boot', description: 'Boots the iOS simulator of mobile:ios-simulator, on macOS (experimental)')]
function ios_boot(): void
{
    if (!context()['macos']) {
        throw new \RuntimeException('An iOS simulator needs macOS, with Xcode.');
    }

    $device = simulator();
    io()->title("Booting {$device['name']}");

    // -b boots it first, if needed
    run(['xcrun', 'simctl', 'bootstatus', $device['udid'], '-b']);
}

/**
 * Opens the app mobile:ios built in a simulator, the newest iPhone of the
 * newest iOS, and waits for PHP to answer a page. Tauri sends what the app
 * writes to the log of the system, in debug builds: it goes next to the
 * screenshot.
 *
 * EXPERIMENTAL: .github/workflows/mobile.yml runs it on a macOS runner.
 */
#[AsTask(name: 'ios-simulator', description: 'Opens the iOS app of a demo in a simulator, waits for PHP to answer a page, and takes a screenshot, on macOS (experimental)')]
function ios_simulator(
    #[AsArgument(description: 'The demo to open, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'The screenshot, from the root of the repository')]
    string $screenshot = 'target/nativella/ios-simulator.png',
    #[AsOption(description: 'The time PHP has to answer, in seconds')]
    int $wait = 120,
): void {
    if (!context()['macos']) {
        throw new \RuntimeException('An iOS simulator needs macOS, with Xcode.');
    }

    $project = demo($demo);
    $identifier = tauri_config($project)['identifier'];
    $root = variable('root_dir');

    if (generated('apple') !== $identifier) {
        throw new \RuntimeException(\sprintf('There is no iOS app of %s: run "castor mobile:ios %s" first.', $project->path, $demo));
    }

    $apps = iterator_to_array(finder()->in("{$root}/crates/nativella/gen/apple/build")->directories()->name('*.app')->sortByModifiedTime(), false);
    $app = end($apps) ?: throw new \RuntimeException(\sprintf('There is no iOS app of %s: run "castor mobile:ios %s" first.', $project->path, $demo));

    $device = simulator();
    io()->title("Opening {$project->path} in {$device['name']}");

    // -b boots it first, if needed
    run(['xcrun', 'simctl', 'bootstatus', $device['udid'], '-b']);
    run(['xcrun', 'simctl', 'install', $device['udid'], $app->getPathname()]);

    // Those of a previous run would pass for this one's
    fs()->remove(["{$root}/{$screenshot}", "{$root}/".log_file($screenshot)]);

    // "<identifier>: <pid>"
    $launched = capture(['xcrun', 'simctl', 'launch', '--terminate-running-process', $device['udid'], $identifier]);
    if (!preg_match('/:\s*(\d+)$/', $launched, $matches)) {
        throw new \RuntimeException(\sprintf('simctl launched the app without saying its PID: "%s".', $launched));
    }
    $pid = $matches[1];

    $output = static fn (): string => capture(['xcrun', 'simctl', 'spawn', $device['udid'], 'log', 'show', '--last', '5m', '--style', 'compact', '--predicate', "processID == {$pid}"]);
    // PID, status and label of each service: a PID while the app runs
    $running = static fn (): bool => 1 === preg_match('/^\d+\s+\S+\s+UIKitApplication:'.preg_quote($identifier, '/').'\[/m', capture(['xcrun', 'simctl', 'spawn', $device['udid'], 'launchctl', 'list']));

    io()->comment("Waiting up to {$wait} seconds for PHP to answer a page...");
    $log = wait_for_a_page($wait, $output, $running);

    // Even if the app is gone: the screenshot shows what is left
    fs()->mkdir(\dirname("{$root}/{$screenshot}"));
    run(['xcrun', 'simctl', 'io', $device['udid'], 'screenshot', "{$root}/{$screenshot}"]);

    check_a_page_answered($screenshot, $log, $running(), $wait);
}

/**
 * The APK "castor mobile:android" makes of a demo.
 *
 * @return string from the root of the repository
 */
function android_apk(string $demo, string $abi): string
{
    return 'target/nativella/android/'.$demo.abi_suffix($abi).'.apk';
}

/**
 * What tells the APKs of an ABI from the others: nothing for arm64-v8a, the
 * ABI of phones.
 */
function abi_suffix(string $abi): string
{
    return 'arm64-v8a' === $abi ? '' : "-{$abi}";
}

/**
 * The adb of ANDROID_HOME, or of PATH.
 */
function adb(): string
{
    $adb = getenv('ANDROID_HOME').'/platform-tools/adb';
    if (is_executable($adb)) {
        return $adb;
    }

    return new ExecutableFinder()->find('adb') ?? throw new \RuntimeException('There is no adb: install the platform tools of the Android SDK, in ANDROID_HOME or in PATH.');
}

/**
 * Reads the output of the app until PHP answered a page, the app stopped, or
 * $wait seconds passed.
 *
 * @param \Closure(): string $output
 * @param \Closure(): bool   $running
 */
function wait_for_a_page(int $wait, \Closure $output, \Closure $running): string
{
    $deadline = time() + $wait;

    while (true) {
        $log = $output();
        if (preg_match(PHP_ANSWERED, $log) || !$running() || time() >= $deadline) {
            return $log;
        }

        sleep(2);
    }
}

/**
 * Where the output of the app goes: next to the screenshot.
 */
function log_file(string $screenshot): string
{
    return preg_replace('/\.png$/', '', $screenshot).'.log';
}

/**
 * Writes the output of the app next to the screenshot, and fails if PHP
 * answered no page.
 */
function check_a_page_answered(string $screenshot, string $log, bool $running, int $wait): void
{
    $logFile = log_file($screenshot);
    fs()->dumpFile(variable('root_dir')."/{$logFile}", $log);

    if (!preg_match(PHP_ANSWERED, $log)) {
        io()->writeln($log);

        throw new \RuntimeException($running ? \sprintf('PHP answered no page within %d seconds: the output of the app is above.', $wait) : 'The app stopped before PHP answered a page: its output is above.');
    }

    io()->success(\sprintf('PHP answered a page. The screenshot: %s, the output of the app: %s.', $screenshot, $logFile));
}

/**
 * The newest iPhone of the newest iOS among the simulators of Xcode.
 *
 * @return array{name: string, udid: string}
 */
function simulator(): array
{
    /** @var array{devices: array<string, list<array{name: string, udid: string}>>} $list */
    $list = json_decode(capture(['xcrun', 'simctl', 'list', 'devices', 'available', '--json']), true, flags: \JSON_THROW_ON_ERROR);

    // Runtimes are named com.apple.CoreSimulator.SimRuntime.iOS-18-2
    $runtimes = array_filter(array_keys($list['devices']), static fn (string $runtime): bool => str_contains($runtime, '.SimRuntime.iOS-'));
    usort($runtimes, static fn (string $a, string $b): int => strnatcmp($b, $a));

    foreach ($runtimes as $runtime) {
        $iPhones = array_filter($list['devices'][$runtime], static fn (array $device): bool => str_starts_with($device['name'], 'iPhone'));
        usort($iPhones, static fn (array $a, array $b): int => strnatcmp($b['name'], $a['name']));

        if ([] !== $iPhones) {
            return $iPhones[0];
        }
    }

    throw new \RuntimeException('Xcode has no iPhone simulator: add one in Xcode, Settings > Components.');
}

/**
 * Downloads a static PHP build for iOS: libphp.a, the libraries it needs and
 * its headers, into mobile/php/dist/<platform>-<php version>/.
 *
 * They come from NativePHP's public builds (https://github.com/NativePHP,
 * MIT) until Nativella builds its own for iOS, as it does for Android.
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
 * Adds libresolv to the Xcode project Tauri generated, and generates it
 * again: PHP calls the resolver of the system (dns_get_record()…), and
 * Xcode, which links the app, has no other way to know it, as Tauri has no
 * setting for libraries.
 */
function link_resolver(string $project): void
{
    $spec = "{$project}/project.yml";
    $yaml = (string) file_get_contents($spec);
    $dependency = "      - sdk: WebKit.framework\n";

    if (!str_contains($yaml, $dependency)) {
        throw new \RuntimeException(\sprintf('%s has no "sdk: WebKit.framework" dependency any more: the template of Tauri changed.', $spec));
    }

    fs()->dumpFile($spec, str_replace($dependency, $dependency."      - sdk: libresolv.tbd\n", $yaml));
    run(['xcodegen', 'generate', '--spec', $spec]);
}

/**
 * The libraries of the PHP build for the simulator, for arm64 only: NativePHP
 * builds them for x86_64 and arm64 in universal archives, which rustc cannot
 * read. Extracted once, with lipo.
 *
 * @return string the directory, from the root of the repository
 */
function simulator_libraries(string $php): string
{
    $root = variable('root_dir');
    $dir = "{$php}/Libraries/iphonesimulator-arm64";

    if (!is_dir("{$root}/{$dir}")) {
        // Complete or absent: an interrupted extraction starts again
        $partial = "{$root}/{$dir}.partial";
        fs()->remove($partial);
        fs()->mkdir($partial);

        foreach (finder()->in("{$root}/{$php}/Libraries/iphonesimulator")->files()->name('*.a') as $library) {
            $thin = "{$partial}/{$library->getFilename()}";
            if ('arm64' === trim(capture(['lipo', '-archs', $library->getPathname()]))) {
                fs()->copy($library->getPathname(), $thin);
            } else {
                run(['lipo', $library->getPathname(), '-thin', 'arm64', '-output', $thin]);
            }
        }

        fs()->rename($partial, "{$root}/{$dir}");
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
