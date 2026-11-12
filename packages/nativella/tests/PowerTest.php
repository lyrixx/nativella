<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\PowerResumed;
use Nativella\Event\PowerSourceChanged;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\Power;
use Nativella\Power\Battery;
use Nativella\Power\BatteryState;
use PHPUnit\Framework\TestCase;

final class PowerTest extends TestCase
{
    public function testALaptopOnBattery(): void
    {
        $client = new FakeClient();
        $power = new Power($client);
        $status = ['onBattery' => true, 'battery' => ['level' => 0.42, 'state' => 'discharging', 'timeToEmpty' => 5400, 'timeToFull' => null]];
        $client->respond('power.status', $status);
        $client->respond('power.status', $status);

        $this->assertTrue($power->isOnBattery());
        $this->assertEquals(new Battery(0.42, BatteryState::Discharging, 5400, null), $power->battery());
    }

    public function testADesktopComputerHasNoBattery(): void
    {
        $power = new Power(new FakeClient());

        $this->assertFalse($power->isOnBattery());
        $this->assertNull($power->battery());
    }

    public function testKeepAwakeGivesTheReason(): void
    {
        $client = new FakeClient();
        $power = new Power($client);

        $power->keepAwake('Exporting the notes');
        $power->allowSleep();

        $this->assertEquals([
            new Call('power.keepAwake', ['reason' => 'Exporting the notes']),
            new Call('power.allowSleep', []),
        ], $client->calls());
    }

    public function testTheEventsOfThePower(): void
    {
        $this->assertEquals(new PowerSourceChanged(true), EventFactory::create('power.sourceChanged', ['onBattery' => true]));
        $this->assertEquals(new PowerResumed(), EventFactory::create('power.resumed', []));
    }

    public function testABatteryStateThePackageDoesNotKnowIsRefused(): void
    {
        $client = new FakeClient();
        $client->respond('power.status', ['onBattery' => false, 'battery' => ['level' => 0.5, 'state' => 'swapping', 'timeToEmpty' => null, 'timeToFull' => null]]);

        $this->expectException(UnexpectedResponseException::class);
        $this->expectExceptionMessage('The runtime sent a battery state the package does not know: "swapping".');

        new Power($client)->battery();
    }
}
