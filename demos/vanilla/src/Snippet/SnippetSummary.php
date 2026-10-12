<?php

namespace App\Snippet;

/**
 * A snippet as the list shows it: a snippet may be a whole imported file, the
 * list only loads the start of its content.
 */
final readonly class SnippetSummary
{
    public function __construct(
        public int $id,
        public string $title,
        /** The first SnippetRepository::EXCERPT_LENGTH characters of the content. */
        public string $excerpt,
        /** Whether the content goes on after the excerpt. */
        public bool $truncated,
        /** In UTC. */
        public \DateTimeImmutable $createdAt,
    ) {
    }
}
