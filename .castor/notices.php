<?php

namespace notices;

use function Castor\context;
use function Castor\finder;
use function Castor\fs;
use function Castor\http_download;
use function Castor\io;
use function Castor\run;
use function Castor\variable;
use function docker\builder_path;
use function docker\builder_platform;
use function docker\docker_compose_run;
use function docker\run_in_builder;

/**
 * The third-party notices: the copyright notices and the licenses of what
 * the executables and the template APK contain, gathered at each build from
 * what it uses. One file per platform: the nativella command embeds it (see
 * crates/nativella/build.rs), `nativella build` writes it next to the
 * applications, the template APK carries it as an asset.
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
    if ($staticPhp) {
        $buildroot = 'static-php/buildroot';
        $sections = php_sections(php_version("{$buildroot}/include/php/main/php_version.h"), "{$buildroot}/license");
    }
    $sections[] = rust_section(target_triple($platform));

    $intro = 'The nativella command, and every application it builds, contain the software below, '
        .'linked statically, under the licenses below. '
        .($staticPhp
            ? 'The libraries of the system they load, such as glibc, libstdc++ and WebKitGTK on Linux, or the frameworks of macOS, are not part of them.'
            : 'They load the PHP of the system, and its libraries, which are not part of them, as the other libraries of the system.');

    return write("target/nativella/notices/THIRD_PARTY_NOTICES-{$platform}.txt", $platform, $intro, $staticPhp, $sections);
}

/**
 * The notices of the template APK, and so of every Android application made
 * of it: the Rust crates of the application library, the PHP of the phone
 * with the libraries it links, and the Java and Kotlin libraries of the
 * Android project.
 *
 * @param string       $php           the PHP of the phone, from the root of the repository (see mobile\fetch_php())
 * @param list<string> $features      the features of the nativella crate the template has
 * @param list<string> $javaLibraries "<group>:<artifact>:<version>" (see gradle_libraries())
 *
 * @return string the file, from the root of the repository
 */
function android(string $php, array $features, array $javaLibraries): string
{
    $sections = php_sections(php_version("{$php}/include/php_version.h"), nativephp_licenses($php), nativephp: true);
    $rust = rust_section('aarch64-linux-android', $features);
    $sections[] = $rust;
    $sections[] = java_section($javaLibraries, str_contains($rust['text'], "\n--- Apache License 2.0: "));

    $intro = 'The template APK of `nativella android`, and every application made of it, contain the software below, '
        .'under the licenses below. The libraries of Android they load, such as its WebView, are not part of them.';

    return write('target/nativella/notices/THIRD_PARTY_NOTICES-android.txt', 'android', $intro, true, $sections);
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
 * The libraries of NativePHP's build of PHP for phones, as static-php-cli
 * names their sources: to dump their licenses, which the build does not
 * ship. Null for NativePHP's own code. A library missing here fails the
 * build, so that none ships without its license.
 */
const NATIVEPHP_LIBRARIES = [
    'libcharset.a' => 'libiconv',
    'libcompat.a' => null,
    'libcrypto.a' => 'openssl',
    'libcurl.a' => 'curl',
    'libglob_compat.a' => null,
    'libiconv.a' => 'libiconv',
    'libonig.a' => 'onig',
    'libphp.a' => 'php-src',
    'libpng16.a' => 'libpng',
    'libsodium.a' => 'libsodium',
    'libsqlite3.a' => 'sqlite',
    'libssl.a' => 'openssl',
    'libxml2.a' => 'libxml2',
    'libz.a' => 'zlib',
];

/**
 * The licenses of NativePHP's build of PHP for phones, which ships none,
 * dumped by static-php-cli from the sources of its libraries: PHP in its
 * version, the libraries in their current one. Once, next to the build.
 *
 * @return string the directory, from the root of the repository
 */
function nativephp_licenses(string $php): string
{
    $root = variable('root_dir');
    $dir = "{$php}/licenses";

    if (is_file("{$root}/{$dir}/php-src/README.REDIST.BINS")) {
        return $dir;
    }

    $sources = [];
    foreach (finder()->in("{$root}/{$php}/staticLibs")->files()->name('*.a') as $library) {
        $name = $library->getFilename();
        if (!\array_key_exists($name, NATIVEPHP_LIBRARIES)) {
            throw new \RuntimeException(\sprintf('NativePHP\'s PHP links %s, which notices\NATIVEPHP_LIBRARIES does not know: add its source, for its license.', $name));
        }
        if (null !== NATIVEPHP_LIBRARIES[$name]) {
            $sources[] = NATIVEPHP_LIBRARIES[$name];
        }
    }
    $sources = array_values(array_unique($sources));
    sort($sources);
    $sources = implode(',', $sources);

    io()->section('Gathering the licenses of the PHP of the phone');

    // static-php-cli works in its working directory: downloads, sources
    $work = 'target/nativella/nativephp-licenses';
    $spc = builder_path(\static_php\spc());
    $version = php_version("{$php}/include/php_version.h");
    fs()->remove("{$root}/{$work}");
    fs()->mkdir("{$root}/{$work}");
    run_in_builder([$spc, 'download', $sources, "--custom-url=php-src:https://www.php.net/distributions/php-{$version}.tar.xz", '--no-motd'], builder_path($work));
    run_in_builder([$spc, 'extract', $sources, '--source-only', '--no-motd'], builder_path($work));
    run_in_builder([$spc, 'dump-license', "--for-sources={$sources}", '--dump-dir='.builder_path("{$dir}.partial"), '--no-motd'], builder_path($work));
    copy_php_src_licenses("{$work}/source/php-src", "{$dir}.partial/php-src");
    fs()->remove("{$root}/{$work}");
    fs()->rename("{$root}/{$dir}.partial", "{$root}/{$dir}", overwrite: true);

    return $dir;
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
 * @return list<array{title: string, text: string}>
 */
function php_sections(string $version, string $licenses, bool $nativephp = false): array
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

    $intro = $nativephp
        ? "NativePHP's build of PHP for phones, https://github.com/NativePHP, links the libraries below, "
            .'and adds an extension and compatibility functions of its own. It ships no license: these are '
            .'the licenses of the sources of the libraries, in their current version, as static-php-cli dumps them.'
        : 'PHP is built by static-php-cli, https://github.com/crazywhalecc/static-php-cli, and linked with the libraries below.';
    $libraries = wordwrap($intro, 79)."\n".$libraries;

    return [
        ['title' => "PHP {$version}", 'text' => $php],
        ['title' => 'The libraries of PHP', 'text' => $libraries],
    ];
}

/**
 * The Java and Kotlin libraries, with the licenses their POM declares, from
 * the cache of Gradle.
 *
 * @param list<string> $libraries "<group>:<artifact>:<version>"
 *
 * @return array{title: string, text: string}
 */
function java_section(array $libraries, bool $apacheTextAbove): array
{
    $cache = (context()['run_on_host'] ? (getenv('GRADLE_USER_HOME') ?: getenv('HOME').'/.gradle') : variable('root_dir').'/.home/.gradle').'/caches/modules-2/files-2.1';

    $intro = 'The Android project of Tauri, and its plugins, use the libraries below, from the Maven repositories '
        .'of Google and Maven Central, under the licenses their POM declares, or the one of their parent POM'
        .($apacheTextAbove ? ': the text of the Apache License 2.0 is above, with the Rust crates.' : '.');
    $text = wordwrap($intro, 79)."\n\n";

    foreach ($libraries as $library) {
        [$group, $artifact, $version] = explode(':', $library, 3);
        $licenses = pom_licenses($cache, $group, $artifact, $version);

        $text .= "{$library}: ".([] === $licenses ? 'no license in its POM' : implode('; ', $licenses))."\n";
    }

    return ['title' => 'The Java and Kotlin libraries', 'text' => $text];
}

/**
 * The licenses the POM of a library declares, or its parent POM, from the
 * cache of Gradle.
 *
 * @return list<string> "<name> (<url>)"
 */
function pom_licenses(string $cache, string $group, string $artifact, string $version): array
{
    $poms = glob("{$cache}/{$group}/{$artifact}/{$version}/*/{$artifact}-{$version}.pom") ?: [];
    if ([] === $poms) {
        return [];
    }

    $document = new \DOMDocument();
    if (!@$document->load($poms[0])) {
        throw new \RuntimeException(\sprintf('%s is not valid XML.', $poms[0]));
    }

    // A POM has <license> elements in <licenses> only
    $licenses = [];
    foreach ($document->getElementsByTagName('license') as $license) {
        $name = trim($license->getElementsByTagName('name')->item(0)->textContent ?? '');
        $url = trim($license->getElementsByTagName('url')->item(0)->textContent ?? '');
        $licenses[] = '' === $url ? $name : "{$name} ({$url})";
    }

    $parent = $document->getElementsByTagName('parent')->item(0);
    if ([] === $licenses && null !== $parent) {
        $coordinate = static fn (string $name): string => trim($parent->getElementsByTagName($name)->item(0)->textContent ?? '');

        return pom_licenses($cache, $coordinate('groupId'), $coordinate('artifactId'), $coordinate('version'));
    }

    return $licenses;
}

/**
 * @param list<array{title: string, text: string}> $sections
 *
 * @return string $file
 */
function write(string $file, string $platform, string $intro, bool $includesPhp, array $sections): string
{
    $title = \sprintf('Third-party notices of Nativella %s, %s', nativella_version(), $platform);
    $text = "{$title}\n".str_repeat('=', \strlen($title))."\n\n".wordwrap($intro, 79)."\n";

    if ($includesPhp) {
        // What the PHP License asks every redistribution to say
        $text .= "\nThis product includes PHP software, freely available from\n<http://www.php.net/software/>.\n";
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
 * The version of the crates, from the root Cargo.toml.
 */
function nativella_version(): string
{
    $manifest = (string) file_get_contents(variable('root_dir').'/Cargo.toml');

    if (!preg_match('{^\[workspace\.package\][^\[]*^version = "([^"]+)"}ms', $manifest, $matches)) {
        throw new \LogicException('The root Cargo.toml has no version in [workspace.package].');
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
