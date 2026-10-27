<?php

namespace Nativella\Updater;

final readonly class Update
{
    public function __construct(
        public string $version,
        public ?string $notes,
        public ?\DateTimeImmutable $date,
    ) {
    }

    /**
     * @param array{version: string, notes: ?string, date: ?string} $data
     */
    public static function fromArray(array $data): self
    {
        $date = null === $data['date'] ? null : \DateTimeImmutable::createFromFormat(\DateTimeInterface::RFC3339, $data['date']);

        return new self($data['version'], $data['notes'], false === $date ? null : $date);
    }
}
