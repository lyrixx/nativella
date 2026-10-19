# Developing Nativella

The repository comes with a Docker stack, driven by [Castor](https://castor.jolicode.com) and based on [jolicode/docker-starter](https://github.com/jolicode/docker-starter). It builds and tests everything without installing PHP, Composer or Rust on your machine, and serves the website and the demos in a browser. The CI runs the same tasks.

Only the desktop application itself runs on your machine by default: a window needs your display. Without Docker, the [host context](#contexts) runs the tasks with the tools of your machine, as the macOS runners of the CI do. None of this is needed to use Nativella: the `nativella` command [builds a project](building.md) on its own.

## Prerequisites

- [Docker](https://docs.docker.com/engine/install/), with Compose v2.
- [Castor](https://castor.jolicode.com/getting-started/installation/) 1.8 or later.
- Optionally, [mkcert](https://github.com/FiloSottile/mkcert#installation), for HTTPS certificates that your browser trusts (run `mkcert -install` once). Without it, the certificates are self-signed.
- To run the desktop application on your machine: Rust and the [system packages](#on-your-machine). Without them, `--docker` runs it in a container (see [below](#the-desktop-application)).

Castor completes its tasks in bash, zsh and fish:

```bash
castor completion | sudo tee /etc/bash_completion.d/castor
```

## The stack

| Service | What |
|---|---|
| `builder` | PHP 8.5 and Composer, the Rust toolchain (stable, with Clippy and rustfmt), `libphp8.5-embed` and its headers, clang, WebKitGTK 4.1 and the other libraries of Tauri. Every command runs there. |
| `android` | The builder, plus a JDK, the Android SDK with an NDK, the Android target of Rust and the Tauri command, for `castor mobile:android`. A few gigabytes, x86_64 only: `castor docker:build` leaves it out, the Android build builds it. |
| `static` | Ubuntu 22.04, with Rust and the libraries of Tauri, but no PHP: the [static PHP](#static-php) and the executables linked against it, for an old glibc. `castor docker:build` leaves it out, `castor static-php:build` builds it. |
| `frontend` | nginx and PHP-FPM: the website, and each demo in a browser. |
| `router` | Traefik, for the domains and HTTPS. |

The containers run with your user ID, on the repository mounted in `/var/www`. Their home directory is `.home/` (ignored by Git): the Composer and cargo caches survive there.

The builder compiles the crates into its own target directory, `target/docker/`: it links the libphp of the image, and the host links its own, so the two builds must not share one.

### Domains

Point the domains at your machine, once:

```bash
echo '127.0.0.1 nativella.test www.nativella.test symfony.nativella.test laravel.nativella.test vanilla.nativella.test' | sudo tee -a /etc/hosts
```

| URL | What |
|---|---|
| `https://nativella.test`, `https://www.nativella.test` | The website: `website/dist/`, built by `castor website:build`. |
| `https://<demo>.nativella.test` | `demos/<demo>/public`, through PHP-FPM. A new demo gets its domain without any configuration (add it to `/etc/hosts`). |

In a browser, the demos run without the native features: there is no window to drive, and no `app.booted` event. That is why `castor migrate` migrates their databases.

The router listens on ports 80, 443 and 8080 (the Traefik dashboard). If another project uses them, set `PROJECT_HTTP_PORT`, `PROJECT_HTTPS_PORT` or `PROJECT_ADMIN_PORT` before `castor start`.

### Git worktrees

In a [git worktree](https://git-scm.com/docs/git-worktree), the stack is isolated from the others: its project name, volumes and networks get the name of the worktree, and its ports are derived from it. `castor docker:ports` shows them.

## Daily tasks

```bash
castor start        # builds the images, installs, builds the website, migrates, starts the stack
castor stop         # stops the stack
castor about        # the URLs
castor              # every task
```

`castor start` runs, in order:

1. `castor docker:build`: the images (the first build takes a while: WebKitGTK and Rust are big).
2. `castor install`: `composer install` in every project, then the tools: the QA tools, and Zensical in `tools/zensical/.venv`. A Laravel project without a `.env` gets one from `.env.example`, with a key.
3. `castor website:build`: the landing page in `website/dist/`, and the documentation in `website/dist/docs/`.
4. `castor migrate`: the databases of the demos, for the frontend.
5. `castor docker:up`: the router and the frontend.

The projects are the directories of `packages/*` and `demos/*`, and `website/`, that have a `composer.json`: the tasks find them at each run. The demos are the directories of `demos/*` with a `public/index.php`.

### Commands in the builder

```bash
castor builder                                                  # a shell, in /var/www
castor builder -- composer require foo/bar -d demos/symfony
castor builder -- bash -c 'cd demos/symfony && bin/console about'
```

`castor builder` starts in `/var/www`, the root of the repository: use `-d` with Composer, or `cd` in the shell, to work in a project.

## Quality

```bash
castor qa                    # everything, fixing the coding standards
castor qa --dry-run          # everything, reporting only: what the CI checks
```

| Task | What |
|---|---|
| `castor qa:cs [--dry-run]` | [PHP CS Fixer](https://cs.symfony.com), Symfony style (`.php-cs-fixer.php`): the `src/` and `tests/` of the packages and demos, the website, the Castor tasks. |
| `castor qa:twig-cs [--dry-run]` | [Twig CS Fixer](https://github.com/VincentLanglet/Twig-CS-Fixer) (`.twig-cs-fixer.php`): the `templates/` of the projects. |
| `castor qa:phpstan [--project=demos/symfony]` | [PHPStan](https://phpstan.org), level 8, on the Castor tasks and on each project. |
| `castor qa:phpunit [--project=demos/symfony]` | The test suite of each project: PHPUnit, or `php artisan test` for a Laravel application. |
| `castor qa:rust` | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`, on the workspace. |
| `castor qa:security-audit` | `composer audit` in each project. |

PHPStan runs from the directory of each project, to find its classes through its own `vendor/`: the projects do not share their dependencies, nor their namespaces (each demo has an `App\`). A project without a PHPStan configuration is analysed with `tools/phpstan/project.neon`, over the directories of its PSR-4 autoloading. One that needs more (an extension, a baseline) adds a `phpstan.dist.neon`:

```neon
includes:
    - ../../tools/phpstan/project.neon

parameters:
    paths:
        - src
        - tests
```

The QA tools live in `tools/<tool>/`, each with its own `composer.json`: `castor qa:install` installs them, and `castor qa:update` updates them. `castor qa:install` installs Zensical too, in `tools/zensical/.venv`, with the versions of `tools/zensical/requirements.txt`, which tells how to upgrade them.

## The desktop application

```bash
castor nativella:run symfony --static            # on your machine, with cargo
castor nativella:run symfony --static --docker   # in a container
```

`nativella:run` builds and runs the `nativella` command on a demo (`cargo run -p nativella -- run demos/<demo>`), the demo installed (`castor install`). `--static` links the [static PHP](#static-php), as the releases do: build it first, with `castor static-php:build`. Without `--static`, the tasks link [the PHP of the system](#the-php-of-the-system).

### On your machine

A recent stable Rust, 1.88 or later ([rustup](https://rustup.rs)), and the libraries Tauri builds against. On Ubuntu 26.04:

```bash
sudo apt install libclang-dev libwebkit2gtk-4.1-dev libxdo-dev librsvg2-dev libayatana-appindicator3-1
```

| Package | What for |
|---|---|
| `libclang-dev` | The PHP bindings are generated with clang (bindgen, through ext-php-rs). |
| `libwebkit2gtk-4.1-dev` | WebKitGTK, the webview of the windows. |
| `libxdo-dev`, `librsvg2-dev` | Libraries Tauri needs on Linux. |
| `libayatana-appindicator3-1` | The [tray](tray.md) icon of the Symfony demo. |

> [!TIP]
> Without `libclang-dev`, bindgen misses the builtin headers of clang (`stddef.h` not found). Pointing it at an extracted `libclang-common-21-dev` works too:
> `BINDGEN_EXTRA_CLANG_ARGS="-isystem …/usr/lib/llvm-21/lib/clang/21/include"`.

### In a container

`--docker` runs it in a container, the builder (the `static` image with `--static`), on your display: the container gets the Wayland socket, the session D-Bus (notifications, tray) and the X11 socket, through the runtime directory of your session (`XDG_RUNTIME_DIR`), without the AppArmor profile of Docker, which the session bus refuses (see `infrastructure/docker/docker-compose.desktop.yml`). Its data directory is then in `.home/.local/share/`.

### Smoke tests

```bash
castor nativella:smoke [symfony] [--static]
```

Serves requests to each demo through the runtime, without any window (`cargo run -p nativella-runtime --example request`): the `app.booted` event, which migrates the database, then the home page. Each demo gets an empty data directory (`NATIVELLA_DATA_DIR`), under `.home/.local/share/nativella-smoke/`.

### Standalone executables

```bash
castor nativella:build [symfony]
castor nativella:package
```

`nativella:build` builds `dist/nativella`, then the standalone executable of each demo (`dist/symfony`…), in release mode, in the builder. Each demo ships staged for production in `target/nativella/stage/`: without its dev dependencies, in the production environment. The executables link the static PHP below, as the releases do, in the `static` image, and carry it; `--no-static` links [the PHP of the system](#the-php-of-the-system) instead. `nativella:package` copies them to `release/`, named after the platform, with their checksums, as the releases publish them. On Linux, it refuses an executable that needs a glibc, or a libstdc++, newer than the one of Ubuntu 22.04: one built with `--no-static`, or outside the `static` image.

### Static PHP

```bash
castor static-php:build
```

Builds PHP as a static library with [static-php-cli](https://github.com/crazywhalecc/static-php-cli), with the embed SAPI and the extensions of `.castor/static-php.php`, into `static-php/buildroot/` (ignored by Git): about 5 minutes, then nothing until the version of PHP, its extensions, static-php-cli or the glibc change.

On Linux, it builds in the `static` image, which the task builds first: Ubuntu 22.04, with Rust and the libraries of Tauri, but no PHP. An executable needs the glibc it was built with, or a newer one: built on Debian 13, like the builder, it would need glibc 2.41, which not even Ubuntu 24.04 has. Built there, it runs on Ubuntu 22.04 and later, Debian 12 and later. static-php-cli builds PHP there with gcc, not with zig, its default: against that glibc, as WebKitGTK is linked against it too, and for the baseline of the architecture: built for the processor of the build machine, it would stop with an illegal instruction on an older one. gcc builds intl and ICU, which are C++, against the libstdc++ that WebKitGTK loads too; zig would build them against a libc++ of its own, which the linker of cargo does not have.

`nativella:build` links it, and so does `--static` on `nativella:run`, `nativella:smoke` and `qa:rust`, in the `static` image too, in a target directory of its own (`target/docker-static/`, `target/static/` on the host): the executables then need no PHP on the machine. That is how macOS gets a PHP, which Homebrew does not ship as a library. There is no `php.ini`: PHP runs with its defaults, plus the settings Nativella sets.

An embed-only PHP has no `php` binary for ext-php-rs to ask `php -i`: the stand-in `mobile/php/php` describes it from its headers, as it does for the PHP of phones.

### The PHP of the system

Without `--static`, `nativella:run`, `nativella:smoke` and `qa:rust` link the PHP of the system, dynamically, and so does `nativella:build` with `--no-static`: in the builder, the `libphp` of its image (Debian 13); on your machine, the one of your distribution. Nothing to build first, which suits the checks: the Linux jobs of the CI run `qa:rust` and `nativella:smoke` this way, and the static PHP in a job of its own. The executables need that PHP on the machine that runs them (`libphp`, its extensions and its `php.ini`), and a glibc as recent as the one they were built with: they are for development only. On your machine, the tasks check that it is there first, and say what is missing.

On your machine, on Ubuntu 26.04, besides the [packages of Tauri](#on-your-machine):

```bash
sudo apt install libphp8.5-embed php8.5-dev php8.5-cli php8.5-sqlite3
```

| Package | What for |
|---|---|
| `libphp8.5-embed` | PHP as a shared library (`libphp.so`), with the embed SAPI. |
| `php8.5-dev` | The headers of PHP, to compile the runtime against them. |
| `php8.5-cli` | `php -i`, which ext-php-rs reads to generate its bindings. Keep it on the same version as the embedded PHP. |
| `php8.5-sqlite3` | SQLite and `pdo_sqlite`, for the [database](database.md). |

Add the extensions an application needs: `php8.5-mbstring`, `php8.5-xml`, `php8.5-intl`… Debian enables an extension for every SAPI, including the embedded one: they show up in `/etc/php/8.5/embed/conf.d/`, next to its `php.ini`. `cargo install --locked --path crates/nativella` then installs a `nativella` command linked against this PHP, in `~/.cargo/bin`.

### Android and iOS

```bash
castor mobile:android symfony [--release]
castor mobile:android-emulator symfony           # after mobile:android, with adb
castor nativella:bundle symfony                  # target/nativella/app.bundle, for --bundle
castor mobile:ios symfony [--bundle=<file>]      # on a Mac
castor mobile:ios-boot                           # on a Mac, the simulator, ahead of mobile:ios-simulator
castor mobile:ios-simulator symfony              # on a Mac, after mobile:ios
```

`mobile:android` builds the `android` image, then the APK in it: it lands in `crates/nativella/gen/android/app/build/outputs/apk/`. `mobile:android-emulator` installs it on the emulator, or the phone, that `adb` sees, on your machine. `mobile:ios` runs on the Mac itself, which needs Xcode and XcodeGen; `nativella:bundle` makes the bundle it compiles, in Docker, for a Mac without it. `mobile:ios-simulator` opens the app it built in a simulator, once booted: `mobile:ios-boot` boots it beforehand, as the CI does while it builds. Both open the app and wait for PHP to answer a page, and fail if it does not: the screenshot and the output of the app land in `target/nativella/`. See [Mobile](mobile.md#what-is-tested).

## Contexts

The tasks run in the `default` context. The workflows use `--context=ci` (through `CASTOR_CONTEXT=ci`): the same stack, without colors in the Compose output.

```bash
castor --context=ci qa --dry-run
```

The `host` context runs on your machine what the others run in the builder: Composer, PHP, cargo, static-php-cli, with the tools of your machine. The macOS runners of the CI, which have no Docker, use it; so can you, without Docker:

```bash
castor --context=host install
castor --context=host static-php:build
castor --context=host qa:rust --static
```

The paths the commands see are then those of the repository on your machine, not `/var/www`: tasks get them from `builder_path()` (`.castor/docker.php`).

There is no `test` context: each test suite sets its own environment (`phpunit.xml`).

## Documentation

The documents of `docs/` read on GitHub as they are, and `castor website:build` turns them into the documentation of the website, with [Zensical](https://zensical.org). A new document goes in the list of `docs/README.md`, which is the navigation; the build fails on a document left out of it, and on a link to a page, a section or a file that does not exist. [`website/README.md`](../website/README.md) tells what Zensical understands on top of Markdown.

## Split repositories

Each package of `packages/` has a read-only repository of its own, which Composer can install it from. The `gitsplit.yml` workflow pushes the history of each one there with [docker-gitsplit](https://github.com/jderusse/docker-gitsplit), on each push to `main` and each `v*` tag. It splits `main` and the release tags: `vX.Y.Z`, with an optional `-alphaN`, `-betaN` or `-rcN`. Changes, issues and pull requests go to this repository.

| Directory | Package | Repository |
|---|---|---|
| `packages/nativella` | `nativella/nativella` | [lyrixx/nativella-php](https://github.com/lyrixx/nativella-php) |
| `packages/symfony` | `nativella/symfony-bundle` | [lyrixx/nativella-symfony-bundle](https://github.com/lyrixx/nativella-symfony-bundle) |
| `packages/laravel` | `nativella/laravel` | [lyrixx/nativella-laravel](https://github.com/lyrixx/nativella-laravel) |

A new package needs its entry in `.gitsplit.yml`, a `LICENSE`, a `README.md` and a `.gitattributes` like the others, and its repository: created empty, without issues, pull requests, projects, wiki or Actions. Then run the workflow by hand. The workflow pushes with the `GH_TOKEN_SUBSPLIT` secret: a fine-grained token, with read and write access to the contents of these repositories only.

## Continuous integration

The workflows of `.github/workflows/` run Castor tasks in the same stack:

| Workflow | When | What |
|---|---|---|
| `ci.yml` | Pull requests and `main`, but not for the documentation alone (Markdown, `website/zensical.yml`, `website/docs-theme/`, `tools/zensical/`); weekly | `castor start`, the PHP checks and tests, the frontend over HTTPS (`castor qa:frontend`); in parallel, `castor qa:rust` and `castor nativella:smoke`, in the builder and on macOS (arm64, the host context, with the static PHP); the static PHP on Linux, in the `static` image: `castor nativella:smoke --static`, then `castor nativella:build` and `castor nativella:package`, as the release does; hadolint on the Dockerfile. |
| `website.yml` | Changes to `website/`, `docs/` or `tools/zensical/` | `castor website:build`, in the host context, with the PHP and the Python of the runner: no image to build. It fails on a broken link; on `main`, deploys `website/dist/` to GitHub Pages: the landing page, and the documentation under `docs/`. Pages must be set to "GitHub Actions" in the settings of the repository. |
| `mobile.yml` | Changes to `crates/`, `mobile/`, `packages/`, `demos/symfony/` or the Docker stack, manually | `castor mobile:android symfony`: the APK as an artifact. Then `castor mobile:android-emulator symfony`, in an Android 16 emulator (x86_64, with KVM); and, experimentally, `castor mobile:ios symfony` on macOS, with the bundle of `castor nativella:bundle symfony`, made by a job of its own so that iOS does not wait for Android, and `castor mobile:ios-simulator symfony`, the simulator booted in the background while the app builds. Their screenshots and the output of the apps as artifacts. |
| `release.yml` | `v*` tags, manually | `castor nativella:build` and `castor nativella:package`: the Linux x86_64 executables, in the `static` image, and the macOS arm64 ones; as artifacts and, for a tag, on the GitHub release. |
| `cache.yml` | Changes to `infrastructure/docker/` on `main` | `castor docker:push`: pushes the build cache of the images to `ghcr.io/lyrixx/nativella`, which the other workflows build from. |
| `gitsplit.yml` | `main`, `v*` tags, manually | Not a Castor task: [docker-gitsplit](https://github.com/jderusse/docker-gitsplit) pushes each package of `packages/` to its [repository](#split-repositories). Each run splits `main` and the release tags as they are when it starts. |
