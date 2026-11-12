<?php

namespace App\Note;

/**
 * A note as the sidebar lists it, without its body: a note may be a whole
 * imported file.
 */
final readonly class NoteSummary
{
    public function __construct(
        public int $id,
        public string $title,
        public bool $pinned,
        public \DateTimeImmutable $updatedAt,
    ) {
    }
}
