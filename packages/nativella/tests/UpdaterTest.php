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

    public function testTheDateMayHaveAFractionOfASecond(): void
    {
        $this->assertEquals(new \DateTimeImmutable('2026-10-27T10:00:00.123Z'), Update::fromArray(['version' => '1.3.0', 'notes' => null, 'date' => '2026-10-27T10:00:00.123Z'])->date);
        $this->assertEquals(new \DateTimeImmutable('2026-10-27T12:00:00.123456+02:00'), Update::fromArray(['version' => '1.3.0', 'notes' => null, 'date' => '2026-10-27T12:00:00.123456789+02:00'])->date);
    }

    public function testAReleaseWithoutDate(): void
    {
        $this->assertNull(Update::fromArray(['version' => '1.3.0', 'notes' => null, 'date' => null])->date);
    }

    public function testADateThatIsNotRfc3339IsRefused(): void
    {
        $this->expectException(\UnexpectedValueException::class);
        $this->expectExceptionMessage('The "pub_date" of the release, "27/10/2026", is not an RFC 3339 date, as "2026-10-27T10:00:00Z".');

        Update::fromArray(['version' => '1.3.0', 'notes' => null, 'date' => '27/10/2026']);
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
