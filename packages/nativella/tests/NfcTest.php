<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Exception\NativeCallException;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\Nfc;
use Nativella\Nfc\Record;
use Nativella\Nfc\TypeNameFormat;
use PHPUnit\Framework\TestCase;

final class NfcTest extends TestCase
{
    public function testARecordOfZeros(): void
    {
        $record = Record::fromArray(['tnf' => 5, 'type' => base64_encode('0'), 'id' => base64_encode('0'), 'payload' => base64_encode('0')]);

        $this->assertSame(['0', '0', '0'], [$record->type, $record->id, $record->payload]);
    }

    public function testBytesThatAreNotBase64AreRefused(): void
    {
        $this->expectException(UnexpectedResponseException::class);
        $this->expectExceptionMessage('The runtime sent bytes of an NFC record that are not base64: "not base64!".');

        Record::fromArray(['tnf' => 1, 'type' => base64_encode('T'), 'id' => '', 'payload' => 'not base64!']);
    }

    public function testTheFakeHasNoNfc(): void
    {
        $nfc = new Nfc(new FakeClient());

        $this->assertFalse($nfc->isAvailable());

        $this->expectException(NativeCallException::class);
        $this->expectExceptionMessage('The native call "nfc.read" failed: NFC is not available');

        $nfc->read();
    }

    public function testATagWithAUriAndAText(): void
    {
        $client = new FakeClient();
        $client->respond('nfc.read', ['tag' => [
            'id' => '04A102FF',
            'technologies' => ['android.nfc.tech.NfcA', 'android.nfc.tech.Ndef'],
            'records' => [
                ['tnf' => 1, 'type' => base64_encode('U'), 'id' => '', 'payload' => base64_encode("\x04notes.example/42")],
                ['tnf' => 1, 'type' => base64_encode('T'), 'id' => '', 'payload' => base64_encode("\x02frÉtagère B")],
            ],
        ]]);

        $tag = new Nfc($client)->read();

        $this->assertSame('04A102FF', $tag->id);
        $this->assertSame('https://notes.example/42', $tag->records[0]->uri());
        $this->assertNull($tag->records[0]->text());
        $this->assertSame('Étagère B', $tag->records[1]->text());
    }

    public function testATextInUtf16(): void
    {
        $record = new Record(TypeNameFormat::WellKnown, 'T', '', "\x82en".mb_convert_encoding('Été', 'UTF-16BE', 'UTF-8'));

        $this->assertSame('Été', $record->text());
    }

    public function testAnAbsoluteUriIsItsType(): void
    {
        $this->assertSame('https://notes.example', new Record(TypeNameFormat::AbsoluteUri, 'https://notes.example', '', '')->uri());
    }

    public function testNoNfcByDefault(): void
    {
        $this->assertFalse(new Nfc(new FakeClient())->isAvailable());
    }
}
