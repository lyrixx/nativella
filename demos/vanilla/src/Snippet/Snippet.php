<?php

namespace App\Snippet;

final readonly class Snippet
{
    public function __construct(
        public int $id,
        public string $title,
        public string $content,
        /** In UTC. */
        public \DateTimeImmutable $createdAt,
    ) {
    }
}
