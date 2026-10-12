<?php

namespace Nativella\Power;

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
        return new self((float) $data['level'], BatteryState::from($data['state']), $data['timeToEmpty'], $data['timeToFull']);
    }
}
