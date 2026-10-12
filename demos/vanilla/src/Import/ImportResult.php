<?php

namespace App\Import;

use App\Snippet\Snippet;

final readonly class ImportResult
{
    /**
     * @param list<Snippet>      $imported
     * @param list<RejectedFile> $rejected
     */
    public function __construct(
        public array $imported,
        public array $rejected,
    ) {
    }

    /**
     * What was imported, as the title of the notification.
     */
    public function summary(): string
    {
        return match (\count($this->imported)) {
            0 => 'No snippet imported',
            1 => \sprintf('Imported “%s”', $this->imported[0]->title),
            default => \sprintf('Imported %d snippets', \count($this->imported)),
        };
    }

    /**
     * Why files were skipped, null when none was.
     */
    public function details(): ?string
    {
        if ([] === $this->rejected) {
            return null;
        }

        return implode("\n", array_map(
            static fn (RejectedFile $file): string => \sprintf('%s: %s', $file->name, $file->reason),
            $this->rejected,
        ));
    }

    /**
     * The summary and the details, for a flash message or a toast.
     */
    public function message(): string
    {
        return implode("\n", array_filter([$this->summary(), $this->details()]));
    }
}
