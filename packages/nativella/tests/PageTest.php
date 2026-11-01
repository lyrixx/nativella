<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Page;
use PHPUnit\Framework\TestCase;

final class PageTest extends TestCase
{
    public function testAnEventForEveryWindow(): void
    {
        $client = new FakeClient();

        new Page($client)->dispatch('note.saved', ['id' => 42]);

        $this->assertEquals(['event' => 'note.saved', 'detail' => (object) ['id' => 42], 'window' => null], $client->calls('page.dispatch')[0]->payload);
    }

    public function testAnEventWithoutDetailForOneWindow(): void
    {
        $client = new FakeClient();

        new Page($client)->dispatch('notes.changed', window: 'main');

        // A JSON object, as event.detail is in the page.
        $this->assertSame('{"event":"notes.changed","detail":{},"window":"main"}', json_encode($client->calls('page.dispatch')[0]->payload));
    }
}
