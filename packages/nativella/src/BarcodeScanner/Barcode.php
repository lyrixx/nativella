<?php

namespace Nativella\BarcodeScanner;

final readonly class Barcode
{
    public function __construct(
        /** What the code holds. */
        public string $content,
        /** Null for a kind of code that Format does not list. */
        public ?Format $format,
    ) {
    }

    /**
     * @param array{content: string, format: string|null} $data
     */
    public static function fromArray(array $data): self
    {
        return new self(
            $data['content'],
            null === $data['format'] ? null : Format::tryFrom($data['format']),
        );
    }
}
