<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\NotificationClicked;
use Nativella\Notification;
use PHPUnit\Framework\TestCase;

final class NotificationTest extends TestCase
{
    public function testAClickGivesTheReferenceBack(): void
    {
        $client = new FakeClient();

        new Notification($client)->send('New message', 'From Alice', reference: 'message-42');

        $this->assertSame(['title' => 'New message', 'body' => 'From Alice', 'reference' => 'message-42'], $client->calls('notification.send')[0]['payload']);
        $this->assertEquals(new NotificationClicked('message-42'), EventFactory::create('notification.clicked', ['reference' => 'message-42']));
    }
}
