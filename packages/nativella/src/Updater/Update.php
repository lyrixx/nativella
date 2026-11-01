<?php

namespace Nativella\Updater;

final readonly class Update
{
    /**
     * An RFC 3339 date, with or without a fraction of a second.
     */
    private const string DATE_PATTERN = '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/i';

    public function __construct(
        public string $version,
        public ?string $notes,
        public ?\DateTimeImmutable $date,
    ) {
    }

    /**
     * @param array{version: string, notes: ?string, date: ?string} $data
     *
     * @throws \UnexpectedValueException when the date is not an RFC 3339 one, as Tauri's updater would
     */
    public static function fromArray(array $data): self
    {
        return new self($data['version'], $data['notes'], null === $data['date'] ? null : self::date($data['date']));
    }

    private static function date(string $date): \DateTimeImmutable
    {
        if (1 !== preg_match(self::DATE_PATTERN, $date)) {
            throw new \UnexpectedValueException(\sprintf('The "pub_date" of the release, "%s", is not an RFC 3339 date, as "2026-10-27T10:00:00Z".', $date));
        }

        // Down to the microsecond: PHP drops the digits after.
        return new \DateTimeImmutable($date);
    }
}
