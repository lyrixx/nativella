# Developing Nativella

The repository comes with a Docker stack, driven by [Castor](https://castor.jolicode.com) and based on [jolicode/docker-starter](https://github.com/jolicode/docker-starter). It builds and tests everything without installing PHP, Composer or Rust on your machine, and serves the website and the demos in a browser. The CI runs the same tasks.

Only the desktop application itself runs on your machine by default, because a window needs your display. Without Docker, the [host context](#contexts) runs the tasks with your machine's tools, as the CI's macOS runners do. None of this is needed to use Nativella, since the `nativella` command [builds a project](building.md) on its own.

## Prerequisites

- [Docker](https://docs.docker.com/engine/install/), with Compose v2.
- [Castor](https://castor.jolicode.com/getting-started/installation/) 1.9 or later.
- Optionally, [mkcert](https://github.com/FiloSottile/mkcert#installation), for HTTPS certificates that your browser trusts (run `mkcert -install` once). Without it, the certificates are self-signed.
- To run the desktop application on your machine: Rust and the [system packages](#on-your-machine). Without them, `--docker` runs it in a container (see [below](#the-desktop-application)).

Castor completes its tasks in bash, zsh and fish:

```bash
castor completion | sudo tee /etc/bash_completion.d/castor
```

## The stack

| Service | What |
|---|---|
| `builder` | PHP 8.5 and Composer, the Rust toolchain (stable, with Clippy and rustfmt), `libphp8.5-embed` and its headers, clang, WebKitGTK 4.1 and the other Tauri libraries. Every command runs there. |
| `android` | The builder, plus a JDK, the Android SDK with an NDK, Rust's Android target and the Tauri command at version `TAURI_CLI_VERSION` (`.castor/mobile.php`), for the template APK of `castor mobile:android`. It is a few gigabytes and x86_64 only, so `castor docker:build` leaves it out and the Android build builds it. |
| `static` | Ubuntu 22.04, with Rust and the Tauri libraries but no PHP. It builds the [static PHP](#static-php) and the executables linked against it, for an old glibc. `castor docker:build` leaves it out, `castor static-php:build` builds it. |
| `frontend` | nginx and PHP-FPM, for the website and each demo in a browser. |
| `router` | Traefik, for the domains and HTTPS. |

The containers run with your user ID, on the repository mounted in `/var/www`. Their home directory is `.home/` (ignored by Git), where the Composer and cargo caches survive.

The builder compiles the crates into its own target directory, `target/docker/`. It links the image's libphp and the host links its own, so the two builds must not share one.

### Domains

Point the domains at your machine, once:

```bash
echo '127.0.0.1 nativella.test www.nativella.test symfony.nativella.test laravel.nativella.test vanilla.nativella.test' | sudo tee -a /etc/hosts
```

| URL | What |
|---|---|
| `https://nativella.test`, `https://www.nativella.test` | The website: `website/dist/`, built by `castor website:build`. |
| `https://<demo>.nativella.test` | `demos/<demo>/public`, through PHP-FPM. A new demo gets its domain without any configuration (add it to `/etc/hosts`). |

In a browser, the demos run without the native features, as there is no window to drive and no `app.booted` event. That is why `castor migrate` migrates their databases.

The router listens on ports 80, 443 and 8080 (the Traefik dashboard, without a password), on `127.0.0.1` only, so the stack is not reachable from the network. If another project uses them, set `PROJECT_HTTP_PORT`, `PROJECT_HTTPS_PORT` or `PROJECT_ADMIN_PORT` before `castor start`, and for the tasks that reach the router (`castor about`, `castor docker:ports`, `castor qa:frontend`).

### Git worktrees

In a [git worktree](https://git-scm.com/docs/git-worktree), the stack is isolated from the others. Its project name, volumes and networks get the worktree's name, and its ports are derived from it. `castor docker:ports` shows them.

## Daily tasks

```bash
castor start        # builds the images, installs, builds the website, migrates, starts the stack
castor stop         # stops the stack
castor about        # the URLs
castor              # every task
```

`castor start` runs, in order:

1. `castor docker:build`: the images (the first build takes a while, because WebKitGTK and Rust are big).
2. `castor install`: `composer install` in every project, then the QA tools, and Zensical in `tools/zensical/.venv`. A Laravel project without a `.env` gets one from `.env.example`, with a key.
3. `castor website:build`: the landing page in `website/dist/`, and the documentation in `website/dist/docs/`.
4. `castor migrate`: the demo databases, for the frontend.
5. `castor docker:up`: the router and the frontend.

The projects are the directories of `packages/*` and `demos/*`, and `website/`, that have a `composer.json`. The tasks find them at each run. The demos are the directories of `demos/*` with a `public/index.php`.

### Commands in the builder

```bash
castor builder                                                  # a shell, in /var/www
castor builder -- composer require foo/bar -d demos/symfony
castor builder -- bash -c 'cd demos/symfony && bin/console about'
castor builder --service=static -- rustc --version              # in the static image
```

`castor builder` starts in `/var/www`, the repository root. Use `-d` with Composer, or `cd` in the shell, to work in a project.

## Quality

```bash
castor qa                    # everything but the frontend, fixing the coding standards
castor qa --dry-run          # the same, reporting only: what the CI checks
```

| Task | What |
|---|---|
| `castor qa:cs [--dry-run]` | [PHP CS Fixer](https://cs.symfony.com), Symfony style (`.php-cs-fixer.php`), on the `src/` and `tests/` of the packages and demos, the website and the Castor tasks. |
| `castor qa:twig-cs [--dry-run]` | [Twig CS Fixer](https://github.com/VincentLanglet/Twig-CS-Fixer) (`.twig-cs-fixer.php`), on each project's `templates/`. |
| `castor qa:phpstan [--project=demos/symfony]` | [PHPStan](https://phpstan.org), level 8, on the Castor tasks and on each project. |
| `castor qa:phpunit [--project=demos/symfony]` | Each project's test suite, with PHPUnit, or `php artisan test` for a Laravel application. |
| `castor qa:phpunit-lowest` | The packages' test suites, with the lowest versions of their dependencies that their constraints allow, in a copy of `packages/`, `target/lowest/packages/`, so their `vendor/` stays as it is. They run on the builder's PHP, 8.5, not the lowest one they allow. |
| `castor qa:rust` | `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo test --locked`, on the workspace. They fail on a `Cargo.lock` that does not match the manifests. |
| `castor qa:security-audit` | `composer audit` in each project. |
| `castor qa:frontend` | The website and each demo through the router, as a browser does, without the native features. Run `castor start` first. |

A task installs a project it checks, or a demo it runs, that is not installed yet, and builds the static PHP that `--static` links, so on a fresh clone, `castor qa` checks every project.

PHPStan runs from each project's directory, to find its classes through its own `vendor/`, because the projects share neither their dependencies nor their namespaces (each demo has an `App\`). A project without a PHPStan configuration is analysed with `tools/phpstan/project.neon`, over its PSR-4 directories. One that needs more (an extension, a baseline) adds a `phpstan.dist.neon`:

```neon
includes:
    - ../../tools/phpstan/project.neon

parameters:
    paths:
        - src
        - tests
```

The QA tools live in `tools/<tool>/`, each with its own `composer.json`. `castor qa:install` installs them, and `castor qa:update` updates them. `castor qa:install` installs Zensical too, in `tools/zensical/.venv`, with the versions in `tools/zensical/requirements.txt`, which tells how to upgrade them.

## The desktop application

```bash
castor nativella:run symfony --static            # on your machine, with cargo
castor nativella:run symfony --static --docker   # in a container
```

`nativella:run` builds and runs the `nativella` command on a demo (`cargo run -p nativella -- run demos/<demo>`), which it installs first if needed. `--static` links the [static PHP](#static-php), as the releases do. `castor static-php:build` builds it, and the task does so first if needed. Without `--static`, the tasks link [the system PHP](#the-php-of-the-system).

### On your machine

You need a recent stable Rust, 1.88 or later ([rustup](https://rustup.rs)), and the libraries Tauri builds against. On Ubuntu 26.04:

```bash
sudo apt install libclang-dev libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev libayatana-appindicator3-1
```

| Package | What for |
|---|---|
| `libclang-dev` | The PHP bindings are generated with clang (bindgen, through ext-php-rs). |
| `libwebkit2gtk-4.1-dev` | WebKitGTK, the windows' webview. |
| `libxdo-dev`, `librsvg2-dev` | Libraries Tauri needs on Linux. |
| `libayatana-appindicator3-1` | The Symfony demo's [tray](tray.md) icon. |

> [!TIP]
> Without `libclang-dev`, bindgen misses clang's builtin headers (`stddef.h` not found). Pointing it at an extracted `libclang-common-21-dev` works too:
> `BINDGEN_EXTRA_CLANG_ARGS="-isystem …/usr/lib/llvm-21/lib/clang/21/include"`.

### In a container

`--docker` runs it in a container, the builder (the `static` image with `--static`), on your display. The container gets the Wayland socket, the session D-Bus (notifications, tray) and the X11 socket, through your session's runtime directory (`XDG_RUNTIME_DIR`), without Docker's AppArmor profile, which the session bus refuses (see `infrastructure/docker/docker-compose.desktop.yml`). Its data directory is then in `.home/.local/share/`.

### Smoke tests

```bash
castor nativella:smoke [symfony] [--static]
```

This serves requests to each demo through the runtime, without any window (`cargo run -p nativella-runtime --example request`): the `app.booted` event, which migrates the database, then the home page. Each demo gets an empty data directory (`NATIVELLA_DATA_DIR`), under `.home/.local/share/nativella-smoke/`.

### Standalone executables

```bash
castor nativella:build [symfony]
castor nativella:package
```

`nativella:build` builds `dist/nativella`, then each demo's standalone executable (`dist/symfony`…), in release mode, in the builder. Each demo ships staged for production in `target/nativella/stage/`, without its dev dependencies and in the production environment. The executables link the static PHP below, as the releases do, in the `static` image, and carry it; `--no-static` links [the system PHP](#the-php-of-the-system) instead. `nativella:package` copies them to `release/`, named after the platform, with their checksums, as the releases publish them. On Linux, it refuses an executable that needs a glibc or a libstdc++ newer than Ubuntu 22.04's, which means one built with `--no-static` or outside the `static` image.

Before building, `nativella:build` gathers the platform's [third-party notices](building.md#licenses) into `target/nativella/notices/` (`.castor/notices.php`): the licenses static-php-cli dumped when it built PHP, those of what php-src bundles, then those of the target's Rust crates, read from their sources by [cargo-about](https://github.com/EmbarkStudios/cargo-about). The task downloads cargo-about into `tools/cargo-about/bin/`, and checks it against the SHA-256 in `.castor/notices.php`. `tools/cargo-about/about.toml` lists the licenses a crate may have, and any other one fails the build. `NATIVELLA_NOTICES` hands the notices to the crate's build, which compresses them into the executable; `nativella build` writes them to `dist/THIRD_PARTY_NOTICES.txt`, and `nativella:package` to `release/THIRD_PARTY_NOTICES-<platform>.txt`. `mobile:android-template` does the same for the template APK, with the Java libraries Gradle resolves, and puts them in its assets.

### Static PHP

```bash
castor static-php:build
```

This builds PHP as a static library with [static-php-cli](https://github.com/crazywhalecc/static-php-cli), with the embed SAPI and the extensions listed in `.castor/static-php.php`, into `static-php/buildroot/` (ignored by Git). It takes about 5 minutes, then does nothing until the PHP version, its extensions, static-php-cli or the glibc change. The task downloads static-php-cli from its release, and checks it against the SHA-256 in `SPC_SHA256`, next to its version.

On Linux, it builds in the `static` image, which the task builds first: Ubuntu 22.04, with Rust and the Tauri libraries but no PHP. An executable needs the glibc it was built with, or a newer one. Built on Debian 13, like the builder, it would need glibc 2.41, which not even Ubuntu 24.04 has. Built in that image, it runs on Ubuntu 22.04 and later, Debian 12 and later. static-php-cli builds PHP there with gcc rather than zig, its default. That builds it against that glibc, as WebKitGTK is linked against it too, and for the architecture's baseline. Built for the build machine's processor, it would stop with an illegal instruction on an older one. gcc builds intl and ICU, which are C++, against the libstdc++ that WebKitGTK loads too; zig would build them against a libc++ of its own, which cargo's linker does not have.

`nativella:build` links it, and so does `--static` on `nativella:run`, `nativella:smoke` and `qa:rust`, in the `static` image too, in a target directory of its own (`target/docker-static/`, `target/static/` on the host). The executables then need no PHP on the machine. That is how macOS gets a PHP, which Homebrew does not ship as a library. It reads only Nativella's php.ini, `crates/runtime/php.ini`, compiled into the runtime, and not the `/usr/local/etc/php/php.ini` static-php-cli builds it to look for, which on users' machines belongs to another PHP.

An embed-only PHP has no `php` binary for ext-php-rs to ask `php -i`, so the stand-in `mobile/php/php` describes it from its headers, as it does for the phones' PHP.

#### The version of PHP

`PHP_RELEASE`, in `.castor/static-php.php`, pins the PHP version, to its patch release, for the desktop and the phones, on every machine and in every CI job. Each buildroot's stamp holds it, and the keys of the static PHP [caches](#caches) hash that file. When it changes, each platform builds the new PHP, whatever its cache holds.

Moving to a new PHP release, security releases above all, is a change of this line, in a pull request of its own, whose workflows build the new PHP on Linux, macOS, Android and iOS. php.net gives the latest release of a branch, `https://www.php.net/releases/index.php?json&version=8.5`; static-php-cli downloads it from there, into `static-php/downloads/`, and again when the version changes.

### PHP for the phones

```bash
castor static-php:android [--abi=x86_64]
castor static-php:ios                           # on a Mac
```

This builds the same PHP for phones: PHP 8.5, embed SAPI, without thread safety, with the extensions of `EXTENSIONS` in `.castor/static-php.php`, the same list as the desktop. static-php-cli only builds for the machine it runs on, and has no support for Android or iOS. It downloads the sources into `static-php/downloads/` (the desktop's sources when both build on the same machine), and `.castor/static-php-mobile.php` cross compiles them with static-php-cli's recipes, so the libraries and their features are the same. An extension added to `EXTENSIONS` needs its `configure` options there, in `CONFIGURE`, or the build stops and says so; a new library needs a recipe. The licenses of PHP and its libraries land in the buildroot's `license/`, which static-php-cli dumps from their sources, as for the desktop. The template's [third-party notices](building.md#licenses) come from there.

`static-php:android` builds in the `android` image, which it builds first, with its NDK's clang, for the template's API level (the `minSdkVersion` of `crates/nativella/tauri.android.conf.json`), into `static-php/android-arm64/buildroot/`, or `android-x86_64/` for emulators on x86_64 computers. It takes a few minutes, starting with ICU and OpenSSL. `static-php:ios` builds on the Mac itself, with Xcode, CMake and pkg-config, for Tauri's default iOS version (15.0), into `static-php/ios-arm64/` and `static-php/ios-simulator-arm64/`. `mobile:android-template` and `mobile:ios` build what they need first.

Each buildroot has a stamp, like the desktop's. The build starts again only when static-php-cli, the PHP version, the extensions, the NDK or Xcode, or `.castor/static-php-mobile.php` change. `--force` starts it again anyway. The sources are extracted in the `source/` next to it, which a successful build removes.

### The PHP of the system

Without `--static`, `nativella:run`, `nativella:smoke` and `qa:rust` link the system PHP dynamically, and so does `nativella:build` with `--no-static`. In the builder, that is the `libphp` of its image (Debian 13); on your machine, your distribution's. There is nothing to build first, which suits the checks: the CI's Linux jobs run `qa:rust` and `nativella:smoke` this way, and the static PHP in a job of its own. The executables need that PHP on the machine that runs them (`libphp`, its extensions and its `php.ini`), and a glibc as recent as the one they were built with, so they are for development only. On your machine, the tasks first check that it is there, and say what is missing.

On your machine, on Ubuntu 26.04, besides the [Tauri packages](#on-your-machine):

```bash
sudo apt install libphp8.5-embed php8.5-dev php8.5-cli php8.5-sqlite3
```

| Package | What for |
|---|---|
| `libphp8.5-embed` | PHP as a shared library (`libphp.so`), with the embed SAPI. |
| `php8.5-dev` | The PHP headers, to compile the runtime against them. |
| `php8.5-cli` | `php -i`, which ext-php-rs reads to generate its bindings. Keep it on the same version as the embedded PHP. |
| `php8.5-sqlite3` | SQLite and `pdo_sqlite`, for the [database](database.md). |

Add the extensions an application needs: `php8.5-mbstring`, `php8.5-xml`, `php8.5-intl`… Debian enables an extension for every SAPI, including the embedded one, so they show up in `/etc/php/8.5/embed/conf.d/`, next to its `php.ini`. `cargo install --locked --path crates/nativella` then installs a `nativella` command linked against this PHP, in `~/.cargo/bin`.

### Android and iOS

```bash
castor mobile:android-template [--release]       # target/nativella/android-template-*.apk
castor mobile:android symfony [--release]        # the template, then the APK of the demo
castor mobile:android-emulator symfony           # after mobile:android, with adb
castor mobile:android symfony --abi=x86_64       # the same for x86_64, which emulators run natively
castor mobile:android-emulator symfony --abi=x86_64
castor mobile:ios-template [--device]            # on a Mac: target/nativella/ios-template-*.zip
castor mobile:ios symfony [--template=<zip>]     # the template, then the app of the demo
castor mobile:ios-simulator symfony              # on a Mac, after mobile:ios
```

`mobile:android-template` builds the `android` image, then [PHP for Android](#php-for-the-phones) and, in that image, the template APK that `nativella android` makes applications from. It lands in `target/nativella/`. `mobile:android` builds it, then makes the demo's APK from it in the builder, in `target/nativella/android/`. With `--abi=x86_64`, both build for x86_64 instead of arm64, for emulators on x86_64 computers, which then run them without ARM translation. Their files end with `-x86_64`. `mobile:android-emulator` installs it on the emulator, or the phone, that `adb` sees, on your machine. `mobile:ios-template` builds PHP for iOS, then a template app that `nativella ios` makes apps from, for the simulator or for devices (`--device`), debug or release (`--release`). It runs on the Mac itself, needs Xcode and XcodeGen, and the template lands in `target/nativella/`. `mobile:ios` builds the simulator template, then makes the demo's app from it in the builder, in `target/nativella/ios/`; with `--template`, it takes a template built beforehand, and runs anywhere. `mobile:ios-simulator` checks the app's signature, then opens it in a simulator, which it boots first if needed. Both open the app and wait for PHP to answer a page, and fail if it does not. The screenshot and the app's output land in `target/nativella/`. See [Mobile](mobile.md#what-is-tested).

The templates build with the Tauri command at version `TAURI_CLI_VERSION`, in `.castor/mobile.php`, which also holds the SHA-256 of its archives. The `android` image installs it, and `castor mobile:tauri-cli` does on your machine, into `tools/tauri-cli/bin/`. It goes with the `tauri` crate of the same minor version. When Dependabot moves the crate to a new one, `mobile:android-template` and `mobile:ios-template` stop, and say so, until `TAURI_CLI_VERSION` follows.

## Contexts

The tasks run in the `default` context. The workflows use `--context=ci` (through `CASTOR_CONTEXT=ci`), which is the same stack without colors in the Compose output.

```bash
castor --context=ci qa --dry-run
```

The `host` context runs on your machine what the others run in the builder: Composer, PHP, cargo, static-php-cli, with your machine's tools. The CI's macOS runners, which have no Docker, use it; so can you, without Docker:

```bash
castor --context=host install
castor --context=host static-php:build
castor --context=host qa:rust --static
```

The paths the commands see are then those of the repository on your machine, not `/var/www`. Tasks get them from `builder_path()` (`.castor/docker.php`).

There is no `test` context: each test suite sets its own environment (`phpunit.xml`).

## Documentation

The documents of `docs/` read on GitHub as they are, and `castor website:build` turns them into the website's documentation, with [Zensical](https://zensical.org). A new document goes in the list of `docs/README.md`, which is the navigation; the build fails on a document left out of it, and on a link to a page, a section or a file that does not exist. [`website/README.md`](../website/README.md) tells what Zensical understands on top of Markdown.

## Split repositories

Each package of `packages/` has a read-only repository of its own, which Composer can install it from. The `gitsplit.yml` workflow pushes each one's history there with [docker-gitsplit](https://github.com/jderusse/docker-gitsplit), on each push to `main` and each `v*` tag. It splits `main` and the release tags: `vX.Y.Z`, with an optional `-alphaN`, `-betaN` or `-rcN`. Changes, issues and pull requests go to this repository.

| Directory | Package | Repository |
|---|---|---|
| `packages/nativella` | `nativella/nativella` | [lyrixx/nativella-php](https://github.com/lyrixx/nativella-php) |
| `packages/symfony` | `nativella/symfony-bundle` | [lyrixx/nativella-symfony-bundle](https://github.com/lyrixx/nativella-symfony-bundle) |
| `packages/laravel` | `nativella/laravel` | [lyrixx/nativella-laravel](https://github.com/lyrixx/nativella-laravel) |

A new package needs its entry in `.gitsplit.yml`, a `LICENSE`, a `README.md` and a `.gitattributes` like the others, and its repository, created empty, without issues, pull requests, projects, wiki or Actions. Then run the workflow by hand. The workflow pushes with the `GH_TOKEN_SUBSPLIT` secret, a fine-grained token with read and write access to the contents of these repositories only.

## Continuous integration

The workflows in `.github/workflows/` run Castor tasks in the same stack. On a pull request, each one runs only on the changes that can break it, listed in the `paths` of its `pull_request` trigger. A new directory, or a new file at the repository root, goes in the `paths` of the workflows it can break, or they skip it. No check is required to merge into `main`, so a skipped workflow blocks nothing.

On `main`, they all run on every push, whatever the paths, except `cache.yml` and `release.yml`. That way `main` catches a path missing from a filter, and pull requests that break only together, before a release is tagged from it. `main` also saves the [caches](#caches) every run restores. The weekly runs ignore the `paths` too.

| Workflow | When | What |
|---|---|---|
| `php.yml` | Changes to the projects (`demos/`, `packages/`, `website/`), `tools/`, the check configurations, the Castor tasks or the Docker stack, but not to the documentation alone (Markdown, `website/zensical.yml`, `website/docs-theme/`, `tools/zensical/`); weekly, for the new advisories; manually | `castor start`, the PHP checks and tests, the frontend over HTTPS (`castor qa:frontend`). |
| `rust.yml` | Changes to the crates (`crates/`, `Cargo.*`), `demos/`, `packages/`, the Castor tasks or the Docker stack, but not to the documentation or the projects' tests alone; weekly; manually | `castor qa:rust` and `castor nativella:smoke`, in the builder. |
| `static.yml` | The same changes as `rust.yml`, and `mobile/php/php`; weekly; manually | The static PHP on Linux, in the `static` image: `castor nativella:smoke --static`, then, except on a pull request, `castor nativella:build` and `castor nativella:package`, as the release does (see below). |
| `macos.yml` | Changes to the crates, `mobile/php/php` or the Castor tasks; weekly; manually | `castor qa:rust --static` and `castor nativella:smoke --static` on macOS (arm64, the host context, with the static PHP). |
| `dockerfile.yml` | Changes to `infrastructure/docker/` | hadolint on the Dockerfile. |
| `website.yml` | Changes to `website/`, `docs/`, `tools/zensical/`, or the repository files the documentation links to | `castor website:build`, in the host context, with the runner's PHP and Python, so there is no image to build. It fails on a broken link. On `main`, it deploys `website/dist/` to GitHub Pages: the landing page, and the documentation under `docs/`. Pages must be set to "GitHub Actions" in the repository settings. |
| `mobile.yml` | Changes to `crates/`, `mobile/`, `packages/`, `demos/symfony/`, `tools/cargo-about/` or the Docker stack, manually | `castor mobile:android symfony`: PHP for Android, cached, the template APK, then the demo's APK, as an artifact. Then `castor mobile:android-emulator symfony`, in an Android 16 emulator (x86_64, with KVM). Experimentally, side by side, `castor static-php:ios`, cached, and `castor mobile:ios-template` on macOS, for the simulator template app; `castor mobile:ios symfony --template=…` on Linux, which makes the demo's app from it, as an artifact; `castor mobile:ios-simulator symfony` on macOS again. Their screenshots and the apps' output are artifacts too. |
| `release.yml` | `v*` tags, manually | `castor nativella:build` and `castor nativella:package`: the Linux x86_64 executables, in the `static` image, and the macOS arm64 ones; `castor mobile:android-template --release`: PHP for Android and the template APK, in the `android` image; `castor mobile:ios-template --release`, with and without `--device`: PHP for iOS and the template apps, on macOS; as artifacts and, for a tag, on the GitHub release, with their checksums, their third-party notices and an [attestation](installation.md#checking-the-download) of their provenance. The release is immutable once published, so the workflow creates it as a draft, uploads the files, then publishes it. Run again on the tag after a failure, it finishes the draft, and fails on a published release. |
| `cache.yml` | Changes to `infrastructure/docker/`, `castor.php` or `.castor/docker.php`, which give the images' build arguments, on `main` | `castor docker:push`: pushes the images' build cache to `ghcr.io/lyrixx/nativella`, which the other workflows build from. |
| `gitsplit.yml` | `main`, `v*` tags, manually | This is not a Castor task. [docker-gitsplit](https://github.com/jderusse/docker-gitsplit) pushes each package of `packages/` to its [repository](#split-repositories). Each run splits `main` and the release tags as they are when it starts. |

A pull request does not build the release executables, which take 3 minutes of `static.yml`. `main` builds them once it is merged, and so does the release, while `castor nativella:smoke --static` already links the runtime against the static PHP. Only `main` catches what those builds alone break: an executable that needs a glibc or a libstdc++ newer than Ubuntu 22.04's (`nativella:package`), the release build of the `nativella` command, linked against the static PHP and WebKitGTK, a demo that fails to stage for production (`mobile.yml` stages the Symfony one), and a desktop crate under a license that `tools/cargo-about/about.toml` does not list. Run by hand on a pull request's branch, `static.yml` builds them before the merge.

The jobs name their runner, `ubuntu-26.04` and `macos-26` (arm64), not `ubuntu-latest` or `macos-latest`, so a runner changes when a pull request of its own moves it, with its CI. Most jobs run in the stack's images, whatever the runner. Two depend on it: Website, with the runner's Python, and the Android emulator, with its KVM and its Android SDK.

The steps the jobs share are composite actions, in `.github/actions/`: `castor`, Castor in the version the workflows run, checked against the SHA-256 of its release; `stack`, Buildx, the login to the registry and Castor, for the jobs that run in the images; `ios-tools`, what builds the iOS template apps; `delete-replaced-caches`, see [Caches](#caches). A local action needs the checkout first, which keeps `persist-credentials: false`, and a workflow lists the ones it uses in its `paths`.

The jobs that build the iOS template apps, in `mobile.yml` and `release.yml`, install XcodeGen from its release, pinned, rather than with Homebrew, as `cargo tauri ios init` would, and the Tauri command with `castor mobile:tauri-cli`. That command also looks for libimobiledevice (`idevicesyslog`), for a phone's logs, and CocoaPods (`pod`), for the pods of an application that has some. The templates use neither, so the jobs put stand-ins, which fail if run, in place of those the runner lacks.

### Caches

Only `main` saves caches, on its pushes and its weekly runs; pull requests and tags restore them, and save none. A cache belongs to the branch that saves it. A pull request's caches would serve that pull request only, and would fill the 10 GB that GitHub gives a repository, evicting those of `main`, the only ones every run can restore. GitHub evicts a cache unused for 7 days too.

The images build from the cache that `cache.yml` pushes on each change of the Dockerfiles. Their base images are pinned by their digest, which Dependabot updates; until then, every job builds the same images, from that cache. Their Rust is the stable of the day `cache.yml` built them, and moves when they are built again, at least with Dependabot's monthly updates. The Rust builds are keyed on its version, as they are on the runner's Rust on macOS.

| Cache | Saved by | What | Keyed on |
|---|---|---|---|
| `cargo-debug-…` | `rust.yml` | The cargo registry and `target/docker/`; the iOS job of `mobile.yml`, which makes the demo's app with `nativella ios`, restores it too | The builder's Rust and the Dockerfile, then `Cargo.lock` |
| `cargo-static-…` | `static.yml` | The cargo registry and `target/docker-static/` | The `static` image's Rust and the Dockerfile, then `Cargo.lock` |
| `static-php-<os>-<arch>-…` | `static.yml`, `macos.yml` | `static-php/buildroot/` | `.castor/static-php.php` |
| `cargo-macOS-ARM64-…` | `macos.yml` | The cargo registry and `target/static/` | The runner's Rust, then `Cargo.lock` |
| `static-php-android-arm64-…` | `mobile.yml` | `static-php/android-arm64/buildroot/`, [PHP for Android](#php-for-the-phones) | The Dockerfile, for its NDK, then `.castor/static-php.php`, `.castor/static-php-mobile.php` and the template's API level |
| `android-…` | `mobile.yml` | The cargo registry, Gradle's dependencies (`.home/.gradle/caches/`, not its distribution, downloaded as fast) and `target/docker/` | The `android` image's Rust and the Dockerfile, then `Cargo.lock` and `.castor/mobile.php` |
| `static-php-ios-…` | `mobile.yml` | `static-php/ios-arm64/buildroot/` and `static-php/ios-simulator-arm64/buildroot/`, PHP for iOS; `release.yml` restores it too | The runner's Xcode, then `.castor/static-php.php` and `.castor/static-php-mobile.php` |
| `ios-…` | `mobile.yml` | The cargo registry and `target/`, except `target/nativella/`, of the job that builds the simulator template app; `release.yml` restores it too, for the downloads | The runner's Rust, then `Cargo.lock` and `.castor/mobile.php` |

A job restores the cache for its key, or else the latest one from `main` with the same first part. For the Rust builds, that means the same Rust, and the same image on Linux. A pull request then falls back to the latest one, whatever its Rust, for the downloads at least, while `main` starts afresh, so that its caches never carry the build of a Rust no longer used. `release.yml` restores the caches of `main` too, and falls back as a pull request does.

`main` saves a cache only when its key changed, and only from a job that succeeded, because a saved cache stays as it is until its key changes and a broken build would stay with it. It then deletes the `main` caches that the new one replaces, with the same first part (`.github/actions/delete-replaced-caches`, with that job's `actions: write` permission). Otherwise GitHub would keep them until the 10 GB are full, then evict the least recently used ones, whatever they are. The static PHP, and PHP for the phones, are saved as soon as they are built, whatever fails next, because their stamp, not the key, says when they build again. Before it saves the crates' build, `castor nativella:prune-target` removes from `target/` what cargo built of the workspace crates. A checkout makes their sources newer than their build, so cargo builds them again anyway. Only their dependencies stay.

The CI's cargo commands build without incremental compilation and without debug info: `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0`, in the workflows' `env`, which `infrastructure/docker/docker-compose.yml` passes to the containers when they are set. Neither helps a build from a fresh checkout, and they made up most of the caches. Without them, the `rust.yml` build weighs a third of what it did, and takes less time. A panic still gives its file and its line; a backtrace, with `RUST_BACKTRACE=1`, only its functions. The job that builds the simulator template app keeps the line tables (`CARGO_PROFILE_DEV_DEBUG=line-tables-only`). Without debug info, cargo tells build scripts that the build is not a debug one (`DEBUG=false`), swift-rs then builds Tauri's Swift in release, and only a debug build of it sends what the app writes to the system log, where `mobile:ios-simulator` reads that PHP answered.
