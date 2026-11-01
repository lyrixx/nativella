<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Exception\NativeCallException;
use PHPUnit\Framework\TestCase;

final class FakeClientTest extends TestCase
{
    public function testItAnswersWhatNothingHappeningAnswers(): void
    {
        $client = new FakeClient();

        $this->assertSame(['confirmed' => false], $client->call('dialog.confirm', ['message' => 'Delete?']));
        $this->assertSame(['path' => '/home/nativella/appData'], $client->call('app.path', ['directory' => 'appData']));
        $this->assertSame([], $client->call('notification.send', ['title' => 'Saved']));
        $this->assertTrue($client->isNative());
    }

    public function testQueuedAnswersComeInOrderThenTheDefault(): void
    {
        $client = new FakeClient();
        $client->respond('dialog.confirm', ['confirmed' => true]);
        $client->respond('dialog.confirm', new NativeCallException('dialog.confirm', new \RuntimeException('No display.')));

        $this->assertSame(['confirmed' => true], $client->call('dialog.confirm'));
        try {
            $client->call('dialog.confirm');
            $this->fail('The queued exception was expected.');
        } catch (NativeCallException $e) {
            $this->assertSame('The native call "dialog.confirm" failed: No display.', $e->getMessage());
        }
        $this->assertSame(['confirmed' => false], $client->call('dialog.confirm'));
    }

    public function testAnAnswerIsForItsMethodOnly(): void
    {
        $client = new FakeClient();
        $client->respond('dialog.ask', ['answer' => true]);

        $this->assertSame(['confirmed' => false], $client->call('dialog.confirm'));
        $this->assertSame(['answer' => true], $client->call('dialog.ask'));
    }

    public function testItRecordsTheCallsEvenThoseThatThrow(): void
    {
        $client = new FakeClient();
        $client->respond('window.close', new NativeCallException('window.close', new \RuntimeException('No window "settings".')));

        $client->call('window.setTitle', ['title' => 'Notes', 'window' => null]);
        try {
            $client->call('window.close', ['window' => 'settings']);
        } catch (NativeCallException) {
        }

        $this->assertEquals([new Call('window.setTitle', ['title' => 'Notes', 'window' => null]), new Call('window.close', ['window' => 'settings'])], $client->calls());
        $this->assertEquals([new Call('window.close', ['window' => 'settings'])], $client->calls('window.close'));
        $this->assertSame([], $client->calls('window.open'));
    }

    public function testResetForgetsTheCallsAndTheQueuedAnswers(): void
    {
        $client = new FakeClient();
        $client->respond('dialog.confirm', ['confirmed' => true]);
        $client->call('notification.send', ['title' => 'Saved']);

        $client->reset();

        $this->assertSame([], $client->calls());
        $this->assertSame(['confirmed' => false], $client->call('dialog.confirm'));
    }
}
