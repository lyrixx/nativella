# Static PHP

By default, `nativella` links the PHP of the system (`libphp8.5-embed`): the
standalone executables of `nativella build` then need it on the machines they
run on. Linked against a static PHP instead, they don't.

```bash
static-php/build.sh                      # about 5 minutes: PHP and its libraries, with static-php-cli
source static-php/env.sh
cargo build --release -p nativella --target-dir target/static
target/static/release/nativella build demos/symfony -o dist/notes
```

`build.sh` downloads [static-php-cli](https://github.com/crazywhalecc/static-php-cli)
and builds PHP with the embed SAPI and the extensions listed in it, into
`buildroot/` (ignored by Git). On Linux, it builds against the glibc of the
system with zig, as WebKitGTK is linked: the executables need a glibc as
recent as the one they were built on, and WebKitGTK.

There is no php.ini: PHP runs with its defaults, and the settings Nativella
sets (session directory, time zone…).

`env.sh` points the build to it: `NATIVELLA_PHP_LIB_DIR` makes the runtime
link every static library of `buildroot/lib` (see `crates/runtime/build.rs`),
and the stand-in `mobile/php/php` describes this PHP to ext-php-rs, since an
embed-only build has no `php` binary to ask.
