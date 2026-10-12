<?php

namespace App\Tests\Import;

use App\Import\RejectedFile;
use App\Import\SnippetImporter;
use App\Snippet\Snippet;
use App\Snippet\SnippetRepository;
use App\Snippet\SnippetSchema;
use Doctrine\DBAL\DriverManager;
use Nativella\Client\FakeClient;
use Nativella\Nativella;
use PHPUnit\Framework\TestCase;

final class SnippetImporterTest extends TestCase
{
    private const string FIXTURES = __DIR__.'/../Fixtures';

    private FakeClient $client;
    private SnippetRepository $repository;
    private SnippetImporter $importer;

    protected function setUp(): void
    {
        $connection = DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true]);
        new SnippetSchema($connection)->migrate();
        $this->client = new FakeClient();
        $this->repository = new SnippetRepository($connection);
        $this->importer = new SnippetImporter(new Nativella($this->client), $this->repository);
    }

    public function testImportFilesSavesEachFileAsASnippet(): void
    {
        $result = $this->importer->importFiles([self::FIXTURES.'/hello.txt', self::FIXTURES.'/notes.md']);

        $this->assertSame([], $result->rejected);
        $this->assertSame(['hello.txt', 'notes.md'], array_map(static fn (Snippet $snippet): string => $snippet->title, $result->imported));
        $this->assertSame("Hello from a file.\n", $this->repository->find($result->imported[0]->id)?->content);
        $this->assertSame("# Notes\n\n- one\n- two\n", $this->repository->find($result->imported[1]->id)?->content);
    }

    public function testImportFilesSendsANotification(): void
    {
        $this->importer->importFiles([self::FIXTURES.'/hello.txt']);

        $this->assertSame(
            [['method' => 'notification.send', 'payload' => ['title' => 'Imported “hello.txt”', 'body' => null, 'reference' => null]]],
            $this->client->calls(),
        );
    }

    public function testImportFilesSkipsWhatIsNotATextFile(): void
    {
        $result = $this->importer->importFiles([
            self::FIXTURES.'/empty.txt',
            self::FIXTURES.'/image.png',
            self::FIXTURES,
            self::FIXTURES.'/deleted.txt',
            self::FIXTURES.'/hello.txt',
        ]);

        $this->assertCount(1, $result->imported);
        $this->assertEquals([
            new RejectedFile('empty.txt', 'The content cannot be empty.'),
            new RejectedFile('image.png', 'The content must be UTF-8 text.'),
            new RejectedFile('Fixtures', 'It is not a file.'),
            new RejectedFile('deleted.txt', 'It is not a file.'),
        ], $result->rejected);
        $this->assertSame([[
            'method' => 'notification.send',
            'payload' => [
                'title' => 'Imported “hello.txt”',
                'body' => "empty.txt: The content cannot be empty.\nimage.png: The content must be UTF-8 text.\nFixtures: It is not a file.\ndeleted.txt: It is not a file.",
                'reference' => null,
            ],
        ]], $this->client->calls());
    }

    public function testNothingImported(): void
    {
        $result = $this->importer->importFiles([self::FIXTURES.'/image.png']);

        $this->assertSame('No snippet imported', $result->summary());
        $this->assertSame("No snippet imported\nimage.png: The content must be UTF-8 text.", $result->message());
    }

    public function testImportChosenFileAsksWithTheFileDialog(): void
    {
        $this->client->respond('dialog.openFile', ['path' => self::FIXTURES.'/notes.md']);

        $result = $this->importer->importChosenFile();

        $this->assertSame('Imported “notes.md”', $result?->summary());
        $this->assertSame('Import a text file', $this->client->calls('dialog.openFile')[0]['payload']['title']);
        $this->assertCount(1, $this->client->calls('notification.send'));
    }

    public function testImportChosenFileDoesNothingWhenTheUserCancels(): void
    {
        $this->assertNull($this->importer->importChosenFile());
        $this->assertSame(['dialog.openFile'], array_column($this->client->calls(), 'method'));
    }

    public function testTheSummaryCountsTheSnippets(): void
    {
        $result = $this->importer->importFiles([self::FIXTURES.'/hello.txt', self::FIXTURES.'/notes.md']);

        $this->assertSame('Imported 2 snippets', $result->summary());
        $this->assertNull($result->details());
    }
}
