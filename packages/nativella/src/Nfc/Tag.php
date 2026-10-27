<?php

namespace Nativella\Nfc;

final readonly class Tag
{
    /**
     * @param list<string> $technologies Android's classes ("android.nfc.tech.NfcA"), or iOS's families ("MiFare")
     * @param list<Record> $records      the NDEF message of the tag, empty when it holds none
     */
    public function __construct(
        /** In hexadecimal, as "04A1B2C3D4"; null when the phone does not tell it. */
        public ?string $id,
        public array $technologies,
        public array $records,
    ) {
    }

    /**
     * @param array{id: ?string, technologies: list<string>, records: list<array{tnf: int, type: string, id: string, payload: string}>} $data
     */
    public static function fromArray(array $data): self
    {
        return new self($data['id'], $data['technologies'], array_map(Record::fromArray(...), $data['records']));
    }
}
