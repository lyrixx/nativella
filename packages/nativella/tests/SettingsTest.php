<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Settings;
use PHPUnit\Framework\TestCase;

final class SettingsTest extends TestCase
{
    public function testAMissingKeyGivesTheDefault(): void
    {
        $this->assertSame('light', new Settings(new FakeClient())->get('theme', 'light'));
    }

    public function testANullValueIsNotMissing(): void
    {
        $client = new FakeClient();
        $client->respond('settings.get', ['value' => null]);

        $this->assertNull(new Settings($client)->get('theme', 'light'));
    }

    public function testAKeyIsDeleted(): void
    {
        $client = new FakeClient();

        new Settings($client)->delete('theme');

        $this->assertEquals([new Call('settings.forget', ['key' => 'theme'])], $client->calls());
    }
}
