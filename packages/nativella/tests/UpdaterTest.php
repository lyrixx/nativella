<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\UpdateFailed;
use Nativella\Event\UpdateInstalled;
use Nativella\Updater;
use Nativella\Updater\Update;
use PHPUnit\Framework\TestCase;

final class UpdaterTest extends TestCase
{
    public function testANewerRelease(): void
    {
        $client = new FakeClient();
        $client->respond('updater.check', ['update' => ['version' => '1.3.0', 'notes' => 'Faster search.', 'date' => '2026-10-27T10:00:00Z']]);
        $client->respond('updater.install', ['version' => '1.3.0']);
        $updater = new Updater($client);

        $this->assertEquals(new Update('1.3.0', 'Faster search.', new \DateTimeImmutable('2026-10-27T10:00:00Z')), $updater->check());
        $this->assertSame('1.3.0', $updater->install());
    }

    public function testTheLatestRelease(): void
    {
        $updater = new Updater(new FakeClient());

        $this->assertNull($updater->check());
        $this->assertNull($updater->install());
    }

    public function testTheEventsOfTheUpdater(): void
    {
        $this->assertEquals(new UpdateInstalled('1.3.0'), EventFactory::create('updater.installed', ['version' => '1.3.0']));
        $this->assertEquals(new UpdateFailed('1.3.0', 'No space left.'), EventFactory::create('updater.failed', ['version' => '1.3.0', 'message' => 'No space left.']));
    }
}
