# Mobile builds

Scripts to build Android applications of the projects of this repository. How it works and how to use them: [docs/mobile.md](../docs/mobile.md).

| Path | What |
|---|---|
| `stage.sh` | Copies a project to `target/nativella/stage/` and prepares it for production. |
| `android/build.sh` | Builds an APK of a project. |
| `android/env.sh` | The environment of an Android build, to source. |
| `ios/build.sh` | Builds an iOS simulator app of a project, on macOS. Experimental: it has never run. |
| `php/fetch.sh` | Downloads a static PHP for Android or iOS into `php/dist/`. |
| `php/php`, `php/php-config` | Describe that PHP to ext-php-rs, in place of the PHP of the computer. |
