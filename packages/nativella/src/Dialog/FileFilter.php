<?php

namespace Nativella\Dialog;

/**
 * Restricts a file dialog to some extensions: new FileFilter('Images', ['png', 'jpg']).
 */
final readonly class FileFilter
{
    /**
     * @param array<string> $extensions without the dot
     */
    public function __construct(
        public string $name,
        public array $extensions,
    ) {
    }

    /**
     * @return array{name: string, extensions: list<string>}
     */
    public function toArray(): array
    {
        return ['name' => $this->name, 'extensions' => array_values($this->extensions)];
    }
}
