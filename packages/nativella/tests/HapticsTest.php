<?php

namespace Nativella\Tests;

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

        $this->assertSame([
            ['method' => 'haptics.vibrate', 'payload' => ['milliseconds' => 300]],
            ['method' => 'haptics.impact', 'payload' => ['impact' => 'light']],
            ['method' => 'haptics.feedback', 'payload' => ['feedback' => 'success']],
            ['method' => 'haptics.selection', 'payload' => []],
        ], $client->calls());
    }
}
