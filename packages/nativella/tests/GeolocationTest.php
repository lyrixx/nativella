<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Geolocation;
use Nativella\Geolocation\Accuracy;
use PHPUnit\Framework\TestCase;

final class GeolocationTest extends TestCase
{
    public function testThePositionOfThePhone(): void
    {
        $client = new FakeClient();
        $client->respond('geolocation.currentPosition', ['position' => [
            'latitude' => 45.75,
            'longitude' => 4.85,
            'accuracy' => 12,
            'altitude' => null,
            'altitudeAccuracy' => null,
            'speed' => 0,
            'heading' => null,
            'timestamp' => 1_760_000_000_123,
        ]]);

        $position = new Geolocation($client)->currentPosition(Accuracy::High, 60);

        $this->assertNotNull($position);
        $this->assertSame([45.75, 4.85, 12.0, null, 0.0], [$position->latitude, $position->longitude, $position->accuracy, $position->altitude, $position->speed]);
        $this->assertSame('2025-10-09T08:53:20+00:00', $position->timestamp->format(\DATE_ATOM));
        $this->assertSame(['accuracy' => 'high', 'maximumAge' => 60], $client->calls('geolocation.currentPosition')[0]['payload']);
    }

    public function testTheUserMayRefuse(): void
    {
        $this->assertNull(new Geolocation(new FakeClient())->currentPosition());
    }
}
