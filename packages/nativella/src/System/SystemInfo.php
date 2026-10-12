<?php

namespace Nativella\System;

final readonly class SystemInfo
{
    public function __construct(
        /** "linux", "macos", "windows"… */
        public string $os,
        /** "unix" or "windows". */
        public string $family,
        public string $arch,
        public string $version,
        public ?string $hostname,
        /** A BCP 47 tag, "fr-FR". */
        public ?string $locale,
    ) {
    }

    /**
     * @param array{os: string, family: string, arch: string, version: string, hostname: ?string, locale: ?string} $data
     */
    public static function fromArray(array $data): self
    {
        return new self($data['os'], $data['family'], $data['arch'], $data['version'], $data['hostname'], $data['locale']);
    }
}
