<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Nfc;
use Nativella\Nfc\Record;
use Nativella\Nfc\TypeNameFormat;
use PHPUnit\Framework\TestCase;

final class NfcTest extends TestCase
{
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
