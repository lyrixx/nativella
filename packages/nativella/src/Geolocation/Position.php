<?php

namespace Nativella\Geolocation;

final readonly class Position
{
    public function __construct(
        /** In degrees. */
        public float $latitude,
        /** In degrees. */
        public float $longitude,
        /** In meters: the radius of the circle the phone is in. */
        public float $accuracy,
        /** In meters, above sea level. */
        public ?float $altitude,
        /** In meters. */
        public ?float $altitudeAccuracy,
        /** In meters per second. */
        public ?float $speed,
        /** In degrees, clockwise from the north. */
        public ?float $heading,
        /** When the phone measured it. */
        public \DateTimeImmutable $timestamp,
    ) {
    }

    /**
     * @param array{latitude: int|float, longitude: int|float, accuracy: int|float, altitude: int|float|null, altitudeAccuracy: int|float|null, speed: int|float|null, heading: int|float|null, timestamp: int} $data
     */
    public static function fromArray(array $data): self
    {
        $optional = static fn (int|float|null $value): ?float => null === $value ? null : (float) $value;

        return new self(
            (float) $data['latitude'],
            (float) $data['longitude'],
            (float) $data['accuracy'],
            $optional($data['altitude']),
            $optional($data['altitudeAccuracy']),
            $optional($data['speed']),
            $optional($data['heading']),
            // Milliseconds since the epoch.
            new \DateTimeImmutable('@'.intdiv($data['timestamp'], 1000)),
        );
    }
}
