<?php

namespace Nativella\System;

final readonly class SystemInfo
{
    public function __construct(
        /** "linux", "macos", "windows", "android" or "ios", as Runtime::platform(). */
        public string $platform,
        /** "unix" or "windows". */
        public string $family,
        public string $arch,
        /** The version of the system. */
        public string $version,
        public ?string $hostname,
        /** A BCP 47 tag, "fr-FR". */
        public ?string $locale,
    ) {
    }

    /**
     * @param array{platform: string, family: string, arch: string, version: string, hostname: ?string, locale: ?string} $data
     */
    public static function fromArray(array $data): self
    {
        return new self($data['platform'], $data['family'], $data['arch'], $data['version'], $data['hostname'], $data['locale']);
    }
}
