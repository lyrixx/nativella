<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Share;
use PHPUnit\Framework\TestCase;

final class ShareTest extends TestCase
{
    public function testItSharesATextOrAFile(): void
    {
        $client = new FakeClient();
        $share = new Share($client);

        $share->text('Groceries: eggs, flour');
        $share->file('/data/notes.csv', 'text/csv');

        $this->assertEquals([
            new Call('share.text', ['text' => 'Groceries: eggs, flour']),
            new Call('share.file', ['path' => '/data/notes.csv', 'mimeType' => 'text/csv']),
        ], $client->calls());
    }
}
