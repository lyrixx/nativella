<?php

namespace App\Snippet;

use Doctrine\DBAL\Connection;
use Doctrine\DBAL\ParameterType;

final readonly class SnippetRepository
{
    /** In characters. */
    public const int EXCERPT_LENGTH = 600;

    /** Dates are stored in UTC, as text: SQLite has no date type. */
    private const string DATE_FORMAT = 'Y-m-d H:i:s';

    public function __construct(
        private Connection $connection,
    ) {
    }

    public function add(string $title, string $content, \DateTimeImmutable $createdAt): Snippet
    {
        $createdAt = $createdAt->setTimezone(new \DateTimeZone('UTC'))->format(self::DATE_FORMAT);

        $this->connection->insert(SnippetSchema::TABLE, [
            'content' => $content,
            'created_at' => $createdAt,
            'title' => $title,
        ]);

        return new Snippet((int) $this->connection->lastInsertId(), $title, $content, self::date($createdAt));
    }

    public function find(int $id): ?Snippet
    {
        $row = $this->connection->createQueryBuilder()
            ->select('content', 'created_at', 'id', 'title')
            ->from(SnippetSchema::TABLE)
            ->where('id = :id')
            ->setParameter('id', $id, ParameterType::INTEGER)
            ->fetchAssociative();

        if (false === $row) {
            return null;
        }

        return new Snippet((int) $row['id'], $row['title'], $row['content'], self::date($row['created_at']));
    }

    /**
     * The newest snippets first, those whose title or content contains $query
     * when it is not empty. As SQLite's LIKE, the search ignores the case of
     * ASCII letters only.
     *
     * @return list<SnippetSummary>
     */
    public function search(string $query, int $limit): array
    {
        $builder = $this->connection->createQueryBuilder()
            ->select(
                'created_at',
                'substr(content, 1, :excerptLength) AS excerpt',
                'id',
                'title',
                'length(content) > :excerptLength AS truncated',
            )
            ->from(SnippetSchema::TABLE)
            ->orderBy('created_at', 'DESC')
            ->addOrderBy('id', 'DESC')
            ->setMaxResults($limit)
            // As an integer: SQLite finds any number smaller than any string.
            ->setParameter('excerptLength', self::EXCERPT_LENGTH, ParameterType::INTEGER);

        if ('' !== $query) {
            // "%" and "_" are wildcards for LIKE: a search for "100%" means them literally.
            $builder
                ->where("title LIKE :pattern ESCAPE '\\' OR content LIKE :pattern ESCAPE '\\'")
                ->setParameter('pattern', '%'.addcslashes($query, '%_\\').'%');
        }

        $snippets = [];
        foreach ($builder->fetchAllAssociative() as $row) {
            $snippets[] = new SnippetSummary(
                (int) $row['id'],
                $row['title'],
                $row['excerpt'],
                (bool) $row['truncated'],
                self::date($row['created_at']),
            );
        }

        return $snippets;
    }

    public function delete(int $id): void
    {
        $this->connection->delete(SnippetSchema::TABLE, ['id' => $id]);
    }

    private static function date(string $value): \DateTimeImmutable
    {
        // "!" leaves no field (microseconds…) to the current time.
        return \DateTimeImmutable::createFromFormat('!'.self::DATE_FORMAT, $value, new \DateTimeZone('UTC'))
            ?: throw new \UnexpectedValueException(\sprintf('The date "%s" of a snippet is corrupted.', $value));
    }
}
