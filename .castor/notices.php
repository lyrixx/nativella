<?php

namespace notices;

use static_php\mobile\Target;

use function Castor\context;
use function Castor\finder;
use function Castor\fs;
use function Castor\http_download;
use function Castor\run;
use function Castor\variable;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\docker_compose_run;
use function docker\run_in_builder;

/**
 * The third-party notices: the copyright notices and the licenses of what
 * the executables and the templates contain, gathered at each build from
 * what it uses. One file per platform: the nativella command embeds it (see
 * crates/nativella/build.rs), `nativella build` writes it next to the
 * applications, the template APK carries it as an asset, the library of the
 * iOS template apps embeds it as the nativella command does.
 */
const FILE = 'THIRD_PARTY_NOTICES.txt';

/** cargo-about, which gathers the licenses of the Rust crates: tools/cargo-about/ */
const CARGO_ABOUT_VERSION = '0.9.2';

/**
 * The notices of the nativella command and of the executables it builds, for
 * the platform of the builder: the Rust crates, and the static PHP of
 * "castor static-php:build" with the libraries it links, unless they link the
 * PHP of the system.
 *
 * @return string the file, from the root of the repository
 */
function desktop(bool $staticPhp): string
{
    $platform = builder_platform();

    $sections = [];
    $lgpl = '';
    if ($staticPhp) {
        $buildroot = 'static-php/buildroot';
        $version = php_version("{$buildroot}/include/php/main/php_version.h");
        $sections = php_sections($version, "{$buildroot}/license", 'PHP is built by static-php-cli, https://github.com/crazywhalecc/static-php-cli, and linked with the libraries below.');

        /** @var list<string> $extensions what static-php-cli built */
        $extensions = json_decode((string) file_get_contents(variable('root_dir')."/{$buildroot}/build-extensions.json"), true, flags: \JSON_THROW_ON_ERROR);
        // GNU libiconv when static-php-cli built it, which dumped its license,
        // in the version of its header. static-php-cli downloads PHP and
        // GNU libiconv from these URLs (PhpSource, config/source.json)
        $iconv = is_file(variable('root_dir')."/{$buildroot}/license/lib_libiconv_0.txt")
            ? \sprintf('GNU libiconv %1$s: https://ftp.gnu.org/gnu/libiconv/libiconv-%1$s.tar.gz', libiconv_version("{$buildroot}/include/iconv.h"))
            : '';
        $lgpl = lgpl($version, $extensions, $iconv, '`nativella build`');
    }
    $sections[] = rust_section(target_triple($platform));

    $intro = 'The nativella command, and every application it builds, contain the software below, '
        .'linked statically, under the licenses below. '
        .($staticPhp
            ? 'The libraries of the system they load, such as glibc, libstdc++ and WebKitGTK on Linux, or the frameworks of macOS, are not part of them.'
            : 'They load the PHP of the system, and its libraries, which are not part of them, as the other libraries of the system.');

    return write("target/nativella/notices/THIRD_PARTY_NOTICES-{$platform}.txt", $platform, $intro, $staticPhp, $lgpl, $sections);
}

/**
 * The notices of the template APK, and so of every Android application made
 * of it: the Rust crates of the application library, the PHP of the phone
 * with the libraries it links, and the Java and Kotlin libraries of the
 * Android project.
 *
 * @param string       $php           the buildroot of the PHP of the phone, from the root of the repository (static_php\mobile\build())
 * @param list<string> $features      the features of the nativella crate the template has
 * @param list<string> $javaLibraries "<group>:<artifact>:<version>" (see gradle_libraries())
 *
 * @return string the file, from the root of the repository
 */
function android(Target $target, string $php, array $features, array $javaLibraries): string
{
    $version = php_version("{$php}/include/php/main/php_version.h");
    $sections = php_sections($version, "{$php}/license", 'PHP is built by Nativella for the phone, from the sources static-php-cli downloads for the desktop, with the features it builds them with there (.castor/static-php-mobile.php), and linked with the libraries below.');
    $rust = rust_section($target->host, $features);
    $sections[] = $rust;
    $sections[] = java_section($javaLibraries, str_contains($rust['text'], "\n--- Apache License 2.0: "));

    $lgpl = lgpl_of($version, $php, '`nativella android`');

    $intro = 'The template APK of `nativella android`, and every application made of it, contain the software below, '
        .'under the licenses below. The libraries of Android they load, such as its WebView, are not part of them.';

    return write("target/nativella/notices/THIRD_PARTY_NOTICES-{$target->name}.txt", $target->name, $intro, true, $lgpl, $sections);
}

/**
 * The notices of a template app of iOS, and so of every app made of it: the
 * Rust crates of the library, and the PHP of the phone with the libraries it
 * links. The Swift code of Tauri and of its plugins comes with their crates.
 *
 * @param string       $rustTarget the target of Rust: aarch64-apple-ios, or aarch64-apple-ios-sim
 * @param string       $php        the buildroot of the PHP of the phone, from the root of the repository (static_php\mobile\build())
 * @param list<string> $features   the features of the nativella crate the template has
 *
 * @return string the file, from the root of the repository
 */
function ios(Target $target, string $rustTarget, string $php, array $features): string
{
    $version = php_version("{$php}/include/php/main/php_version.h");
    $sections = php_sections($version, "{$php}/license", 'PHP is built by Nativella for the phone, from the sources static-php-cli downloads for the desktop, with the features it builds them with there (.castor/static-php-mobile.php), and linked with the libraries below.');
    $sections[] = rust_section($rustTarget, $features);

    $intro = 'The template app of `nativella ios`, and every app made of it, contain the software below, '
        .'under the licenses below. The frameworks of iOS they load, such as WebKit, are not part of them.';

    return write("target/nativella/notices/THIRD_PARTY_NOTICES-{$target->name}.txt", $target->name, $intro, true, lgpl_of($version, $php, '`nativella ios`'), $sections);
}

/**
 * The code under the GNU LGPL 2.1 that the PHP of a phone links, from its
 * build (lgpl()).
 *
 * @param string $php    the buildroot of the PHP of the phone, from the root of the repository
 * @param string $repack the command that packs an application again
 */
function lgpl_of(string $version, string $php, string $repack): string
{
    // The extensions of the build, from its configure command
    preg_match_all("{'--(?:enable|with)-([\\w-]+)}", (string) file_get_contents(variable('root_dir')."/{$php}/include/php/main/build-defs.h"), $matches);
    // GNU libiconv, which the build downloads from there (config/source.json
    // of static-php-cli), in the version of its header
    $iconv = is_file(variable('root_dir')."/{$php}/license/lib_libiconv_0.txt")
        ? \sprintf('GNU libiconv %1$s: https://ftp.gnu.org/gnu/libiconv/libiconv-%1$s.tar.gz', libiconv_version("{$php}/include/iconv.h"))
        : '';

    return lgpl($version, $matches[1], $iconv, $repack);
}

/**
 * The libraries of php-src under the GNU LGPL 2.1, by the extension that
 * brings them: its README.REDIST.BINS.
 */
const PHP_LGPL_LIBRARIES = [
    'bcmath' => 'libbcmath',
    'mbstring' => 'libmbfl',
];

/**
 * The code under the GNU LGPL 2.1 that PHP links statically, with the source
 * of each, and how to link it again, which the LGPL asks to make possible.
 *
 * @param list<string> $extensions the extensions of PHP
 * @param string       $iconv      GNU libiconv and its source, '' when PHP links none
 * @param string       $repack     the command that packs an application again
 */
function lgpl(string $phpVersion, array $extensions, string $iconv, string $repack): string
{
    $components = [];
    $libraries = array_values(array_intersect_key(PHP_LGPL_LIBRARIES, array_flip($extensions)));
    if ([] !== $libraries) {
        $components[] = \sprintf('%s, in PHP %s: https://www.php.net/distributions/php-%2$s.tar.xz', implode(' and ', $libraries), $phpVersion);
    }
    if ('' !== $iconv) {
        $components[] = $iconv;
    }

    if ([] === $components) {
        return '';
    }

    $text = "Under the GNU Lesser General Public License 2.1, linked statically, from\nthese sources:\n";
    foreach ($components as $component) {
        $text .= wordwrap("- {$component}", 79, "\n  ")."\n";
    }

    // Broken by hand: wordwrap() would split the command
    return $text.\sprintf("To link them again: Nativella builds from its sources,\n%s (see docs/development.md), and\n%s packs an application again.\n", workspace_package('repository'), $repack);
}

/**
 * From GNU libiconv's iconv.h: #define _LIBICONV_VERSION 0x0113, for 1.19.
 */
function libiconv_version(string $header): string
{
    $content = (string) file_get_contents(variable('root_dir')."/{$header}");

    if (!preg_match('{^#define _LIBICONV_VERSION 0x([0-9a-f]+)}mi', $content, $matches)) {
        throw new \RuntimeException(\sprintf('%s defines no _LIBICONV_VERSION.', $header));
    }

    $version = (int) hexdec($matches[1]);

    return \sprintf('%d.%d', $version >> 8, $version & 0xFF);
}

/**
 * Copies the license files of php-src that its LICENSE leaves out, next to
 * the license files static-php-cli writes: README.REDIST.BINS, on the
 * libraries php-src bundles, and the files it refers to.
 */
function copy_php_src_licenses(string $phpSrc, string $to): void
{
    $root = variable('root_dir');
    $redist = (string) file_get_contents("{$root}/{$phpSrc}/README.REDIST.BINS");

    fs()->remove("{$root}/{$to}");
    fs()->copy("{$root}/{$phpSrc}/README.REDIST.BINS", "{$root}/{$to}/README.REDIST.BINS");

    preg_match_all('{\bsee (\S+/(?:LICENSE|COPYING)\S*)}', $redist, $matches);
    foreach (array_unique($matches[1]) as $path) {
        // uriparser's COPYING is named after its license, COPYING.BSD-3-Clause
        $files = glob("{$root}/{$phpSrc}/{$path}*") ?: [];
        if ([] === $files) {
            throw new \RuntimeException(\sprintf('php-src has no %s, which its README.REDIST.BINS refers to.', $path));
        }

        foreach ($files as $file) {
            fs()->copy($file, "{$root}/{$to}/".substr($file, \strlen("{$root}/{$phpSrc}/")));
        }
    }
}

/**
 * The Java and Kotlin libraries an Android build packages: the runtime
 * classpath of a variant of the application, as Gradle resolves it.
 *
 * @param array<string, string> $environment of the Gradle build
 *
 * @return list<string> "<group>:<artifact>:<version>", sorted
 */
function gradle_libraries(string $project, string $configuration, array $environment, ?string $service): array
{
    $c = context()->withQuiet()->withData(['docker_compose_run_environment' => $environment], recursive: false);
    $process = docker_compose_run(['./gradlew', '-q', ':app:dependencies', '--configuration', $configuration], $c, $service, workDir: builder_path($project));
    if (!$process->isSuccessful()) {
        throw new \RuntimeException(\sprintf("Gradle could not list the dependencies of %s:\n%s", $project, $process->getErrorOutput()));
    }

    $libraries = [];
    foreach (explode("\n", $process->getOutput()) as $line) {
        // "+--- androidx.webkit:webkit:1.14.0", "\--- org.jetbrains:annotations:13.0 -> 23.0.0 (*)",
        // "+--- androidx.lifecycle:lifecycle-runtime -> 2.10.0", with the version Gradle resolved:
        // not "project :tauri-android", nor the constraints (c), nor what was not resolved (n)
        if (preg_match('{--- ([\w.\-]+):([\w.\-]+)(?::[\w.\-]+ -> |:| -> )([\w.\-]+)( \([cn]\))?}', $line, $match) && !isset($match[4])) {
            $libraries[] = "{$match[1]}:{$match[2]}:{$match[3]}";
        }
    }
    $libraries = array_values(array_unique($libraries));
    sort($libraries);

    if ([] === $libraries) {
        throw new \RuntimeException(\sprintf("Gradle listed no dependency of %s:\n%s", $project, $process->getOutput()));
    }

    return $libraries;
}

/**
 * cargo-about, for the platform of the builder, in tools/cargo-about/bin/.
 *
 * @return string from the root of the repository
 */
function cargo_about(): string
{
    $target = match (builder_platform()) {
        'linux-aarch64' => 'aarch64-unknown-linux-musl',
        'linux-x86_64' => 'x86_64-unknown-linux-musl',
        'macos-aarch64' => 'aarch64-apple-darwin',
        default => throw new \RuntimeException(\sprintf('cargo-about has no build for %s.', builder_platform())),
    };
    $binary = 'tools/cargo-about/bin/cargo-about-'.CARGO_ABOUT_VERSION;
    $root = variable('root_dir');

    if (!is_file("{$root}/{$binary}")) {
        $archive = "{$root}/tools/cargo-about/bin/cargo-about.tar.gz";
        http_download(\sprintf('https://github.com/EmbarkStudios/cargo-about/releases/download/%1$s/cargo-about-%1$s-%2$s.tar.gz', CARGO_ABOUT_VERSION, $target), $archive);
        run(['tar', 'xzf', $archive, '-C', \dirname($archive), '--strip-components=1', \sprintf('cargo-about-%s-%s/cargo-about', CARGO_ABOUT_VERSION, $target)], context: context()->withQuiet());
        fs()->rename(\dirname($archive).'/cargo-about', "{$root}/{$binary}");
        fs()->remove($archive);
    }

    return $binary;
}

/**
 * The Rust crates the nativella crate is built with for $target, and their
 * licenses, by cargo-about (tools/cargo-about/about.toml): not the build
 * dependencies, which build scripts use and the build leaves out.
 *
 * @param list<string> $features
 *
 * @return array{title: string, text: string}
 */
function rust_section(string $target, array $features = []): array
{
    $json = "target/nativella/notices/rust-{$target}.json";
    fs()->mkdir(\dirname(variable('root_dir')."/{$json}"));

    // cargo-about reads the licenses from the sources of the crates,
    // offline, for the same notices on every machine
    run_in_builder(['cargo', 'fetch', '--locked']);
    run_in_builder([
        builder_path(cargo_about()), 'generate', '--format', 'json', '--frozen', '--fail',
        '--config', builder_path('tools/cargo-about/about.toml'),
        '--manifest-path', builder_path('crates/nativella/Cargo.toml'),
        '--target', $target,
        ...([] === $features ? [] : ['--features', implode(' ', $features)]),
        '--output-file', builder_path($json),
    ]);

    /** @var array{licenses: list<array{name: string, text: string, used_by: list<array{crate: array{name: string, version: string}}>}>} $about */
    $about = json_decode((string) file_get_contents(variable('root_dir')."/{$json}"), true, flags: \JSON_THROW_ON_ERROR);

    $text = "The source code of each crate is on https://crates.io, in the version given.\n"
        ."The standard library of Rust is linked in too: MIT or Apache-2.0, see\nhttps://github.com/rust-lang/rust/blob/master/COPYRIGHT.\n";
    foreach ($about['licenses'] as $license) {
        $crates = array_map(static fn (array $use): string => "{$use['crate']['name']} {$use['crate']['version']}", $license['used_by']);
        $text .= heading("{$license['name']}: ".implode(', ', $crates)).trim($license['text'])."\n";
    }

    return ['title' => 'The Rust crates', 'text' => $text];
}

/**
 * PHP, then the libraries it links, from the license files that
 * static-php-cli dumped in $licenses: <kind>_<name>_<index>.txt, the
 * license of php-src being src_php-src_0.txt, and php-src/ with the
 * licenses of what php-src bundles (copy_php_src_licenses()).
 *
 * @param string $builtBy how PHP is built, and from what
 *
 * @return list<array{title: string, text: string}>
 */
function php_sections(string $version, string $licenses, string $builtBy): array
{
    $root = variable('root_dir');
    $php = '';
    $libraries = '';

    foreach (finder()->in("{$root}/{$licenses}")->files()->depth(0)->name('*.txt')->sortByName() as $file) {
        if (!preg_match('{^(ext|lib|src)_(.+)_(\d+)\.txt$}', $file->getFilename(), $matches)) {
            throw new \RuntimeException(\sprintf('%s is not a license file of static-php-cli.', $file->getPathname()));
        }

        $text = trim($file->getContents())."\n";
        if ('php-src' === $matches[2]) {
            $php .= $text;
            continue;
        }

        $name = 'ext' === $matches[1] ? "The {$matches[2]} extension of PHP" : $matches[2];
        $libraries .= heading($name).$text;
    }

    if ('' === $php) {
        throw new \RuntimeException(\sprintf('%s has no license of php-src (src_php-src_0.txt).', $licenses));
    }

    $bundled = '';
    foreach (finder()->in("{$root}/{$licenses}/php-src")->files()->sortByName() as $file) {
        $bundled .= heading("php-src/{$file->getRelativePathname()}").trim($file->getContents())."\n";
    }

    $php .= "\nphp-src bundles the libraries below: its README.REDIST.BINS lists them,\nwith the files it refers to.\n".$bundled;

    $libraries = wordwrap($builtBy, 79)."\n".$libraries;

    return [
        ['title' => "PHP {$version}", 'text' => $php],
        ['title' => 'The libraries of PHP', 'text' => $libraries],
    ];
}

/**
 * The Java and Kotlin libraries, with the licenses their POM declares, from
 * the cache of Gradle: read by mobile/pom-licenses.php, with the PHP of the
 * builder, as the static binary of Castor, which the CI runs, has no XML
 * extension.
 *
 * @param list<string> $libraries "<group>:<artifact>:<version>"
 *
 * @return array{title: string, text: string}
 */
function java_section(array $libraries, bool $apacheTextAbove): array
{
    $process = docker_compose_run(['php', builder_path('mobile/pom-licenses.php'), ...$libraries], context()->withQuiet(), workDir: builder_path());
    if (!$process->isSuccessful()) {
        throw new \RuntimeException(\sprintf("The licenses of the Java libraries cannot be read:\n%s", $process->getErrorOutput().$process->getOutput()));
    }
    /** @var array<string, list<string>> $licenses */
    $licenses = json_decode($process->getOutput(), true, flags: \JSON_THROW_ON_ERROR);

    $intro = 'The Android project of Tauri, and its plugins, use the libraries below, from the Maven repositories '
        .'of Google and Maven Central, under the licenses their POM declares, or the one of their parent POM'
        .($apacheTextAbove ? ': the text of the Apache License 2.0 is above, with the Rust crates.' : '.');
    $text = wordwrap($intro, 79)."\n\n";

    foreach ($libraries as $library) {
        $text .= "{$library}: ".([] === $licenses[$library] ? 'no license in its POM' : implode('; ', $licenses[$library]))."\n";
    }

    return ['title' => 'The Java and Kotlin libraries', 'text' => $text];
}

/**
 * @param string                                   $lgpl     the code under the LGPL and its sources (lgpl()), '' when there is none
 * @param list<array{title: string, text: string}> $sections
 *
 * @return string $file
 */
function write(string $file, string $platform, string $intro, bool $includesPhp, string $lgpl, array $sections): string
{
    $title = \sprintf('Third-party notices of Nativella %s, %s', workspace_package('version'), $platform);
    $text = "{$title}\n".str_repeat('=', \strlen($title))."\n\n".wordwrap($intro, 79)."\n";

    if ($includesPhp) {
        // What the PHP License asks every redistribution to say
        $text .= "\nThis product includes PHP software, freely available from\n<http://www.php.net/software/>.\n";
    }

    if ('' !== $lgpl) {
        $text .= "\n{$lgpl}";
    }

    $text .= "\nContents:\n";
    foreach ($sections as $index => $section) {
        $text .= \sprintf("  %d. %s\n", $index + 1, $section['title']);
    }

    foreach ($sections as $index => $section) {
        $heading = \sprintf('%d. %s', $index + 1, $section['title']);
        $text .= "\n\n".str_repeat('=', 79)."\n{$heading}\n".str_repeat('=', 79)."\n\n".$section['text'];
    }

    fs()->dumpFile(variable('root_dir')."/{$file}", $text);

    return $file;
}

/**
 * The heading of a license, wrapped.
 */
function heading(string $title): string
{
    return "\n".wordwrap("--- {$title} ---", 79)."\n\n";
}

/**
 * From a php_version.h: #define PHP_VERSION "8.5.11".
 */
function php_version(string $header): string
{
    $content = (string) file_get_contents(variable('root_dir')."/{$header}");

    if (!preg_match('{^#define PHP_VERSION "([^"]+)"}m', $content, $matches)) {
        throw new \RuntimeException(\sprintf('%s defines no PHP_VERSION.', $header));
    }

    return $matches[1];
}

/**
 * A key of [workspace.package] in the root Cargo.toml: the version or the
 * repository of the crates.
 */
function workspace_package(string $key): string
{
    $manifest = (string) file_get_contents(variable('root_dir').'/Cargo.toml');

    if (!preg_match('{^\[workspace\.package\][^\[]*^'.preg_quote($key).' = "([^"]+)"}ms', $manifest, $matches)) {
        throw new \LogicException(\sprintf('The root Cargo.toml has no %s in [workspace.package].', $key));
    }

    return $matches[1];
}

/**
 * The Rust target of a platform of the builder.
 */
function target_triple(string $platform): string
{
    return match ($platform) {
        'linux-aarch64' => 'aarch64-unknown-linux-gnu',
        'linux-x86_64' => 'x86_64-unknown-linux-gnu',
        'macos-aarch64' => 'aarch64-apple-darwin',
        'macos-x86_64' => 'x86_64-apple-darwin',
        default => throw new \LogicException(\sprintf('builder_platform() names no platform %s.', $platform)),
    };
}
