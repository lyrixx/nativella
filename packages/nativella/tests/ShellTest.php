<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Shell;
use PHPUnit\Framework\TestCase;

final class ShellTest extends TestCase
{
    public function testItHandsOverToTheApplicationsOfTheSystem(): void
    {
        $client = new FakeClient();
        $shell = new Shell($client);

        $shell->openUrl('mailto:hello@example.com');
        $shell->openPath('/home/me/notes.md');
        $shell->showInFolder('/home/me/notes.md');
        $shell->trash('/home/me/old.md');

        $this->assertEquals([
            new Call('shell.openUrl', ['url' => 'mailto:hello@example.com']),
            new Call('shell.openPath', ['path' => '/home/me/notes.md']),
            new Call('shell.showInFolder', ['path' => '/home/me/notes.md']),
            new Call('shell.trash', ['path' => '/home/me/old.md']),
        ], $client->calls());
    }
}
