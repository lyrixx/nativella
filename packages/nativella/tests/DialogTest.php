<?php

namespace Nativella\Tests;

use Nativella\Client\FakeClient;
use Nativella\Dialog;
use Nativella\Dialog\FileFilter;
use Nativella\Dialog\MessageKind;
use PHPUnit\Framework\TestCase;

final class DialogTest extends TestCase
{
    public function testAskReturnsTheAnswer(): void
    {
        $client = new FakeClient();
        $client->respond('dialog.ask', ['answer' => true]);

        $this->assertTrue(new Dialog($client)->ask('Delete?', 'Notes', MessageKind::Warning));
        $this->assertSame(['message' => 'Delete?', 'title' => 'Notes', 'kind' => 'warning'], $client->calls('dialog.ask')[0]['payload']);
    }

    public function testACancelledFileDialogReturnsNull(): void
    {
        $this->assertNull(new Dialog(new FakeClient())->openFile());
        $this->assertSame([], new Dialog(new FakeClient())->openFiles());
    }

    public function testSaveFileSendsTheFilters(): void
    {
        $client = new FakeClient();
        $client->respond('dialog.saveFile', ['path' => '/home/me/note.md']);

        $path = new Dialog($client)->saveFile(fileName: 'note.md', filters: [new FileFilter('Markdown', ['md'])]);

        $this->assertSame('/home/me/note.md', $path);
        $this->assertSame([
            'title' => null,
            'directory' => null,
            'fileName' => 'note.md',
            'filters' => [['name' => 'Markdown', 'extensions' => ['md']]],
        ], $client->calls('dialog.saveFile')[0]['payload']);
    }
}
