<?php

namespace App\Note;

use App\Entity\Note;

final readonly class ImportResult
{
    /**
     * @param list<Note>   $notes   the imported notes, not flushed yet
     * @param list<string> $skipped the names of the files that could not be imported
     */
    public function __construct(
        public array $notes,
        public array $skipped,
    ) {
    }

    public function summary(): string
    {
        $summary = match (\count($this->notes)) {
            0 => 'No note imported.',
            1 => \sprintf('Imported "%s".', $this->notes[0]->getTitle()),
            default => \sprintf('Imported %d notes.', \count($this->notes)),
        };

        if ([] !== $this->skipped) {
            $summary .= \sprintf(' Skipped %s: only Markdown and text files up to 1 MB are supported.', implode(', ', $this->skipped));
        }

        return $summary;
    }
}
