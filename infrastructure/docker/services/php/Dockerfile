# hadolint global ignore=DL3008

FROM debian:13.7-slim AS php-base

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
        gnupg \
    && curl -sSLo /tmp/debsuryorg-archive-keyring.deb https://packages.sury.org/debsuryorg-archive-keyring.deb \
    && dpkg -i /tmp/debsuryorg-archive-keyring.deb \
    && echo "deb [signed-by=/usr/share/keyrings/deb.sury.org-php.gpg] https://packages.sury.org/php/ trixie main" > /etc/apt/sources.list.d/sury.list \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash-completion \
        procps \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

ARG PHP_VERSION

# The demos store their data in SQLite
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        "php${PHP_VERSION}-apcu" \
        "php${PHP_VERSION}-bcmath" \
        "php${PHP_VERSION}-cli" \
        "php${PHP_VERSION}-common" \
        "php${PHP_VERSION}-curl" \
        "php${PHP_VERSION}-gd" \
        "php${PHP_VERSION}-iconv" \
        "php${PHP_VERSION}-intl" \
        "php${PHP_VERSION}-mbstring" \
        "php${PHP_VERSION}-sqlite3" \
        "php${PHP_VERSION}-uuid" \
        "php${PHP_VERSION}-xml" \
        "php${PHP_VERSION}-zip" \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

# Every PHP setting of every image: each stage enables its own ones with phpenmod
COPY php/mods-available/ /etc/php/${PHP_VERSION}/mods-available/

ENV PHP_VERSION=${PHP_VERSION}
ENV HOME=/home/app
ENV COMPOSER_MEMORY_LIMIT=-1

WORKDIR /var/www

FROM php-base AS frontend

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        nginx \
        "php${PHP_VERSION}-fpm" \
        runit \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/* \
    && rm -r "/etc/php/${PHP_VERSION}/fpm/pool.d/"

RUN useradd -s /bin/false nginx

# The container runs as the host user, so the socket directory must be writable by anyone
COPY php/fpm/php-fpm.conf /etc/php/${PHP_VERSION}/fpm/php-fpm.conf
COPY nginx/nginx.conf /etc/nginx/nginx.conf
COPY nginx/conf.d/ /etc/nginx/conf.d/
RUN mkdir -p /var/run/php && chmod 1777 /var/run/php
RUN rm -rf /etc/service/
COPY runit/ /etc/service/
RUN mkdir -p /etc/service/nginx/supervise /etc/service/php-fpm/supervise \
    && chmod 777 /etc/service/*/supervise/

RUN phpenmod app-default \
    && phpenmod -s fpm app-fpm

EXPOSE 8080

CMD ["runsvdir", "-P", "/etc/service"]

FROM php-base AS builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Default toys; rsync stages the projects for production (mobile/stage.sh)
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        "git" \
        "make" \
        "php${PHP_VERSION}-dev" \
        "rsync" \
        "unzip" \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

# What the Rust crates build and link against: PHP as a library (libphp, its
# headers and php-config come with php-dev), clang for the PHP bindings
# (bindgen, through ext-php-rs), and the libraries of Tauri on Linux
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        libayatana-appindicator3-dev \
        libclang-dev \
        "libphp${PHP_VERSION}-embed" \
        librsvg2-dev \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        pkg-config \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

# Rust, for every user: the container runs as the host's raw UID
ARG RUST_VERSION=stable
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="/usr/local/cargo/bin:$PATH"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | CARGO_HOME=/usr/local/cargo sh -s -- -y --no-modify-path --profile minimal \
        --default-toolchain "${RUST_VERSION}" --component clippy,rustfmt \
    && rustc --version

# The cargo registry lives in the mounted home (.home/), to survive the
# container. The build goes to its own target directory: the host links
# another libphp, and cargo would reuse the bindings generated for one with the
# other.
ENV CARGO_HOME=/home/app/.cargo
ENV CARGO_TARGET_DIR=/var/www/target/docker

# Config
RUN phpenmod app-default \
    && phpenmod -s cli app-builder

# Composer
COPY --from=composer/composer:2.10.0 /usr/bin/composer /usr/bin/composer

# Autocompletion
ADD https://raw.githubusercontent.com/symfony/symfony/refs/heads/7.4/src/Symfony/Component/Console/Resources/completion.bash /tmp/completion.bash

# Composer symfony/console version is too old, and doest not support "API version feature", so we remove it
# Hey, while we are at it, let's add some more completion
RUN sed /tmp/completion.bash \
        -e "s/{{ COMMAND_NAME }}/composer/g" \
        -e 's/"-a{{ VERSION }}"//g' \
        -e "s/{{ VERSION }}/1/g"  \
        > /etc/bash_completion.d/composer \
    && sed /tmp/completion.bash \
        -e "s/{{ COMMAND_NAME }}/console/g" \
        -e "s/{{ VERSION }}/1/g"  \
        > /etc/bash_completion.d/console

# What static-php-cli needs to build PHP and its libraries (castor
# static-php:build): its doctor would install them with sudo
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        autopoint \
        bison \
        cmake \
        flex \
        re2c \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

# Python, for Zensical, which builds the documentation: `castor install` puts
# it in a virtual environment, tools/zensical/.venv
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        python3-venv \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

# Third party tools
ENV PATH="$PATH:/var/www/tools/bin"

# We can't rely on \u: the container runs as the host's raw UID, which has no
# /etc/passwd entry, so bash would print "I have no name!" instead.
RUN cat >> /etc/bash.bashrc <<'EOF'
. /etc/bash_completion

PS1='\[\e[01;33m\]builder \[\e[00;32m\]\w\[\e[0m\] '
EOF

WORKDIR /var/www

# The builder, plus what an Android build needs (castor mobile:android): a JDK,
# the Android SDK with an NDK, the Android targets of Rust and the Tauri
# command. Linux x86_64 only: Google ships no NDK for Linux on ARM.
FROM builder AS android

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        default-jdk-headless \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

ENV JAVA_HOME=/usr/lib/jvm/default-java

# The versions the Android project of Tauri asks for (crates/nativella/gen/android):
# Gradle installs what is missing, so the SDK is writable by the user of the
# container, the host's raw UID
ARG ANDROID_CMDLINE_TOOLS=16111833
ARG ANDROID_NDK=29.0.14206865
ENV ANDROID_HOME=/opt/android-sdk
ENV NDK_HOME=/opt/android-sdk/ndk/${ANDROID_NDK}
RUN curl -sSfL -o /tmp/cmdline-tools.zip "https://dl.google.com/android/repository/commandlinetools-linux-${ANDROID_CMDLINE_TOOLS}_latest.zip" \
    && mkdir -p "${ANDROID_HOME}/cmdline-tools" \
    && unzip -q /tmp/cmdline-tools.zip -d "${ANDROID_HOME}/cmdline-tools" \
    && mv "${ANDROID_HOME}/cmdline-tools/cmdline-tools" "${ANDROID_HOME}/cmdline-tools/latest" \
    && "${ANDROID_HOME}/cmdline-tools/latest/bin/android" --no-metrics --sdk="${ANDROID_HOME}" sdk install \
        "build-tools;36.0.0" \
        "ndk;${ANDROID_NDK}" \
        "platform-tools" \
        "platforms;android-37.0" \
    && chmod -R a+rwX "${ANDROID_HOME}" \
    && rm -rf /tmp/*

# The four Android targets of Rust: `cargo tauri android init` adds them all,
# and rustup is not writable in the container
ARG TAURI_CLI=2.12.2
RUN rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android \
    && curl -sSfL "https://github.com/tauri-apps/tauri/releases/download/tauri-cli-v${TAURI_CLI}/cargo-tauri-x86_64-unknown-linux-gnu.tgz" \
        | tar xz -C /usr/local/bin cargo-tauri \
    && cargo tauri --version

# Builds what Nativella distributes for Linux: PHP as a static library (castor
# static-php:build) and the executables linked against it (--static on the
# tasks that build the crates). On Ubuntu 22.04, for its glibc, 2.35: an
# executable runs on the glibc it was built with, or a newer one. Built on
# Debian 13, it would need glibc 2.41, which not even Ubuntu 24.04 has. No
# PHP here: the projects are staged in the builder.
FROM ubuntu:22.04 AS static

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG DEBIAN_FRONTEND=noninteractive

# What the Rust crates build with: clang for the PHP bindings (bindgen,
# through ext-php-rs), the libraries of Tauri on Linux, OpenSSL. Then what
# static-php-cli needs to build PHP and its libraries: its doctor would
# install them with sudo.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        autoconf \
        automake \
        autopoint \
        bison \
        build-essential \
        bzip2 \
        ca-certificates \
        clang \
        cmake \
        curl \
        flex \
        git \
        libayatana-appindicator3-dev \
        libclang-dev \
        librsvg2-dev \
        libssl-dev \
        libtool \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        patch \
        pkg-config \
        re2c \
        unzip \
        xz-utils \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* /usr/share/doc/*

# Rust, for every user: the container runs as the host's raw UID
ARG RUST_VERSION=stable
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="/usr/local/cargo/bin:$PATH"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | CARGO_HOME=/usr/local/cargo sh -s -- -y --no-modify-path --profile minimal \
        --default-toolchain "${RUST_VERSION}" --component clippy,rustfmt \
    && rustc --version

# As in the builder: the cargo registry in the mounted home (.home/), the
# build in a target directory of its own
ENV HOME=/home/app
ENV CARGO_HOME=/home/app/.cargo
ENV CARGO_TARGET_DIR=/var/www/target/docker-static

WORKDIR /var/www
