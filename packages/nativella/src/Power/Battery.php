<?php

namespace Nativella\Power;

use Nativella\Exception\UnexpectedResponseException;

final readonly class Battery
{
    public function __construct(
        /** From 0 (empty) to 1 (full). */
        public float $level,
        public BatteryState $state,
        /** In seconds, when the system has an estimate. */
        public ?int $timeToEmpty,
        /** In seconds, when the system has an estimate. */
        public ?int $timeToFull,
    ) {
    }

    /**
     * @param array{level: int|float, state: string, timeToEmpty: ?int, timeToFull: ?int} $data
     */
    public static function fromArray(array $data): self
    {
        return new self((float) $data['level'], BatteryState::tryFrom($data['state']) ?? throw UnexpectedResponseException::unknownValue('battery state', $data['state']), $data['timeToEmpty'], $data['timeToFull']);
    }
}
