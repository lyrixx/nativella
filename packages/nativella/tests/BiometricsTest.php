<?php

namespace Nativella\Tests;

use Nativella\Biometrics;
use Nativella\Client\FakeClient;
use PHPUnit\Framework\TestCase;

final class BiometricsTest extends TestCase
{
    public function testTheUserProvesWhoTheyAre(): void
    {
        $client = new FakeClient();
        $biometrics = new Biometrics($client);
        $client->respond('biometrics.authenticate', ['authenticated' => true]);

        $this->assertTrue($biometrics->authenticate('Show the private notes'));
        $this->assertFalse($biometrics->authenticate('Show the private notes'), 'The fake answers that the user cancelled by default.');
        $this->assertSame(['reason' => 'Show the private notes', 'title' => null], $client->calls('biometrics.authenticate')[0]['payload']);
    }
}
