<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Haptics;
use Nativella\Haptics\Feedback;
use Nativella\Haptics\Impact;
use PHPUnit\Framework\TestCase;

final class HapticsTest extends TestCase
{
    public function testThePhoneVibrates(): void
    {
        $client = new FakeClient();
        $haptics = new Haptics($client);

        $haptics->vibrate();
        $haptics->impact(Impact::Light);
        $haptics->feedback(Feedback::Success);
        $haptics->selection();

        $this->assertEquals([
            new Call('haptics.vibrate', ['milliseconds' => 300]),
            new Call('haptics.impact', ['impact' => 'light']),
            new Call('haptics.feedback', ['feedback' => 'success']),
            new Call('haptics.selection', []),
        ], $client->calls());
    }
}
