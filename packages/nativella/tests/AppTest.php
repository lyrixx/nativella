<?php

namespace Nativella\Tests;

use Nativella\App;
use Nativella\Client\FakeClient;
use PHPUnit\Framework\TestCase;

final class AppTest extends TestCase
{
    public function testTheThirdPartyNotices(): void
    {
        $client = new FakeClient();
        $app = new App($client);
        $client->respond('app.thirdPartyNotices', ['text' => "Third-party notices of Nativella\n"]);

        $this->assertSame("Third-party notices of Nativella\n", $app->thirdPartyNotices());
        // A development build of Nativella has none
        $this->assertNull($app->thirdPartyNotices());
    }
}
