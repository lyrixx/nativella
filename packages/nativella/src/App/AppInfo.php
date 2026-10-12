<?php

namespace Nativella\App;

final readonly class AppInfo
{
    public function __construct(
        public string $name,
        public string $version,
        public string $identifier,
        public string $nativellaVersion,
    ) {
    }

    /**
     * @param array{name: string, version: string, identifier: string, nativellaVersion: string} $data
     */
    public static function fromArray(array $data): self
    {
        return new self($data['name'], $data['version'], $data['identifier'], $data['nativellaVersion']);
    }
}
