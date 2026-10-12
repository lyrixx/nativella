<?php

namespace App\Tests\Snippet;

use App\Snippet\Snippet;
use App\Snippet\SnippetRepository;
use App\Snippet\SnippetSchema;
use App\Snippet\SnippetSummary;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\DriverManager;
use PHPUnit\Framework\TestCase;

final class SnippetRepositoryTest extends TestCase
{
    private Connection $connection;
    private SnippetRepository $repository;

    protected function setUp(): void
    {
        $this->connection = DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true]);
        new SnippetSchema($this->connection)->migrate();
        $this->repository = new SnippetRepository($this->connection);
    }

    public function testAddedSnippetsCanBeFound(): void
    {
        $added = $this->repository->add('SSH', 'ssh -A deploy@example.com', new \DateTimeImmutable('2026-10-11 09:30:15.123456', new \DateTimeZone('UTC')));

        $found = $this->repository->find($added->id);

        $this->assertEquals($added, $found);
        // Stored to the second, in UTC.
        $this->assertEquals(new Snippet($added->id, 'SSH', 'ssh -A deploy@example.com', new \DateTimeImmutable('2026-10-11 09:30:15', new \DateTimeZone('UTC'))), $found);
    }

    public function testDatesAreStoredInUtc(): void
    {
        $added = $this->repository->add('Paris', 'Bonjour', new \DateTimeImmutable('2026-10-11 11:30:00', new \DateTimeZone('Europe/Paris')));

        $this->assertSame('2026-10-11 09:30:00', $this->connection->fetchOne('SELECT created_at FROM snippet'));
        $this->assertSame('2026-10-11T09:30:00+00:00', $this->repository->find($added->id)?->createdAt->format(\DATE_ATOM));
    }

    public function testFindReturnsNullForAMissingSnippet(): void
    {
        $this->assertNull($this->repository->find(42));
    }

    public function testACorruptedDateIsAnError(): void
    {
        $this->connection->insert(SnippetSchema::TABLE, ['title' => 'Broken', 'content' => 'x', 'created_at' => 'yesterday']);

        $this->expectException(\UnexpectedValueException::class);
        $this->expectExceptionMessage('The date "yesterday" of a snippet is corrupted.');

        $this->repository->find((int) $this->connection->lastInsertId());
    }

    public function testSearchListsTheNewestFirst(): void
    {
        $this->add('Old', '2026-01-01 00:00:00');
        $this->add('New', '2026-03-01 00:00:00');
        $this->add('Middle', '2026-02-01 00:00:00');

        $this->assertSame(['New', 'Middle', 'Old'], $this->titles($this->repository->search('', 10)));
    }

    public function testSearchIsLimited(): void
    {
        $this->add('Old', '2026-01-01 00:00:00');
        $this->add('New', '2026-03-01 00:00:00');

        $this->assertSame(['New'], $this->titles($this->repository->search('', 1)));
    }

    public function testSearchMatchesTitlesAndContentsIgnoringTheCase(): void
    {
        $this->repository->add('Git log', 'git log --oneline', new \DateTimeImmutable());
        $this->repository->add('Reply', 'Thanks, I will look at the logs.', new \DateTimeImmutable());
        $this->repository->add('Address', '1 rue de la Paix', new \DateTimeImmutable());

        $this->assertSame(['Reply', 'Git log'], $this->titles($this->repository->search('LOG', 10)));
    }

    public function testSearchTakesWildcardsLiterally(): void
    {
        $this->repository->add('Percent', 'Done at 100%', new \DateTimeImmutable());
        $this->repository->add('Underscore', 'snake_case', new \DateTimeImmutable());
        $this->repository->add('Backslash', 'C:\Users', new \DateTimeImmutable());
        $this->repository->add('Other', '1000 and snakeXcase', new \DateTimeImmutable());

        $this->assertSame(['Percent'], $this->titles($this->repository->search('100%', 10)));
        $this->assertSame(['Underscore'], $this->titles($this->repository->search('e_c', 10)));
        $this->assertSame(['Backslash'], $this->titles($this->repository->search('C:\U', 10)));
    }

    public function testSearchOnlyLoadsAnExcerptOfLongContents(): void
    {
        $long = str_repeat('é', SnippetRepository::EXCERPT_LENGTH).'the end';
        $this->repository->add('Long', $long, new \DateTimeImmutable());
        $this->repository->add('Short', 'Short enough', new \DateTimeImmutable());

        [$short, $truncated] = $this->repository->search('', 10);

        $this->assertSame('Short enough', $short->excerpt);
        $this->assertFalse($short->truncated);
        $this->assertSame(str_repeat('é', SnippetRepository::EXCERPT_LENGTH), $truncated->excerpt);
        $this->assertTrue($truncated->truncated);
    }

    public function testDelete(): void
    {
        $kept = $this->repository->add('Kept', 'Kept', new \DateTimeImmutable());
        $deleted = $this->repository->add('Deleted', 'Deleted', new \DateTimeImmutable());

        $this->repository->delete($deleted->id);

        $this->assertNull($this->repository->find($deleted->id));
        $this->assertNotNull($this->repository->find($kept->id));
    }

    private function add(string $title, string $createdAt): void
    {
        $this->repository->add($title, 'Content of '.$title, new \DateTimeImmutable($createdAt, new \DateTimeZone('UTC')));
    }

    /**
     * @param list<SnippetSummary> $snippets
     *
     * @return list<string>
     */
    private function titles(array $snippets): array
    {
        return array_map(static fn (SnippetSummary $snippet): string => $snippet->title, $snippets);
    }
}
