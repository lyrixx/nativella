<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Exception\ErrorKind;
use Nativella\Exception\NativeCallException;
use PHPUnit\Framework\Attributes\DataProvider;
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
        $client->fail('dialog.confirm', ErrorKind::Failed, 'No display.');

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
        $client->fail('window.close', ErrorKind::NotFound, 'There is no "settings" window.');

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

    public function testWhatJsonCannotHoldFailsAsInNativella(): void
    {
        $client = new FakeClient();

        try {
            $client->call('clipboard.writeText', ['text' => "Gr\xFC\xDFe"]);
            $this->fail('A NativeCallException was expected.');
        } catch (NativeCallException $e) {
            $this->assertSame('The native call "clipboard.writeText" failed: Malformed UTF-8 characters, possibly incorrectly encoded', $e->getMessage());
        }

        $this->assertSame([], $client->calls());
    }

    #[DataProvider('provideKinds')]
    public function testItFailsWithEveryKind(ErrorKind $kind): void
    {
        $client = new FakeClient();
        $client->fail('window.close', $kind, 'It failed.');

        try {
            $client->call('window.close');
            $this->fail('A NativeCallException was expected.');
        } catch (NativeCallException $e) {
            $this->assertSame($kind, $e->kind);
            $this->assertSame('window.close', $e->method);
        }
    }

    /**
     * @return iterable<string, array{ErrorKind}>
     */
    public static function provideKinds(): iterable
    {
        foreach (ErrorKind::cases() as $kind) {
            yield $kind->value => [$kind];
        }
    }

    public function testReadingATagFailsWhereNfcIsNotAvailable(): void
    {
        try {
            new FakeClient()->call('nfc.read');
            $this->fail('A NativeCallException was expected.');
        } catch (NativeCallException $e) {
            $this->assertSame(ErrorKind::Unavailable, $e->kind);
        }
    }
}
