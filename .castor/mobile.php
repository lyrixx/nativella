<?php

namespace mobile;

use Castor\Attribute\AsArgument;
use Castor\Attribute\AsOption;
use Castor\Attribute\AsTask;
use Symfony\Component\Process\ExecutableFinder;

use function Castor\capture;
use function Castor\context;
use function Castor\io;
use function Castor\run;
use function Castor\variable;
use function project\demo;

/**
 * On the host, with mobile/android/build.sh: the builder has no Android SDK.
 * The script stages and bundles the demo with the PHP, Composer and nativella
 * command of the host, then cross-compiles it (see docs/mobile.md).
 */
#[AsTask(description: 'Builds an Android APK (arm64) of a demo, on the host')]
function android(
    #[AsArgument(description: 'The demo to build, e.g. symfony', autocomplete: 'project\complete_demo_names')]
    string $demo,
    #[AsOption(description: 'Build a release APK, still to be signed, instead of a debug one')]
    bool $release = false,
): int {
    $project = demo($demo);

    if ($missing = missing_android_prerequisites()) {
        throw new \RuntimeException(\sprintf("The Android build needs, on the host (see docs/mobile.md):\n - %s", implode("\n - ", $missing)));
    }

    io()->title("Building an APK of {$project->path}");

    $command = ['mobile/android/build.sh', $project->path];
    if ($release) {
        $command[] = '--release';
    }

    return (int) run($command, context: context()->withAllowFailure()->withWorkingDirectory(variable('root_dir')))->getExitCode();
}

/**
 * @return list<string> what is missing, and how to get it
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

    foreach (['cargo', 'composer', 'php', 'rsync', 'rustup'] as $command) {
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
