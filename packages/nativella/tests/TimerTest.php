<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\TimerFired;
use Nativella\Timer;
use PHPUnit\Framework\TestCase;

final class TimerTest extends TestCase
{
    public function testTimersAreNamed(): void
    {
        $client = new FakeClient();
        $timer = new Timer($client);

        $timer->every('sync', 300);
        $timer->after('reminder', 60);
        $timer->cancel('sync');

        $this->assertEquals([
            new Call('timer.every', ['name' => 'sync', 'seconds' => 300.0]),
            new Call('timer.after', ['name' => 'reminder', 'seconds' => 60.0]),
            new Call('timer.cancel', ['name' => 'sync']),
        ], $client->calls());
    }

    public function testTheEventCarriesTheName(): void
    {
        $this->assertEquals(new TimerFired('sync'), EventFactory::create('timer.fired', ['name' => 'sync']));
    }
}
