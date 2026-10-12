<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Clipboard;
use PHPUnit\Framework\TestCase;

final class ClipboardTest extends TestCase
{
    public function testImagesTravelAsBase64(): void
    {
        $client = new FakeClient();
        $clipboard = new Clipboard($client);
        $png = "\x89PNG\r\n\x1a\n";

        $clipboard->writeImage($png);
        $client->respond('clipboard.readImage', ['png' => base64_encode($png)]);

        $this->assertSame(['png' => base64_encode($png)], $client->calls('clipboard.writeImage')[0]['payload']);
        $this->assertSame($png, $clipboard->readImage());
    }

    public function testAClipboardWithoutImageGivesNull(): void
    {
        $this->assertNull(new Clipboard(new FakeClient())->readImage());
    }
}
