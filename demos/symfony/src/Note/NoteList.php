<?php

namespace App\Note;

/**
 * The notes of the sidebar.
 *
 * @implements \IteratorAggregate<int, NoteSummary>
 */
final readonly class NoteList implements \Countable, \IteratorAggregate
{
    /**
     * @param list<NoteSummary> $summaries
     * @param bool              $hasMore   whether more notes match than the list holds
     */
    public function __construct(
        public array $summaries,
        public bool $hasMore,
    ) {
    }

    public function count(): int
    {
        return \count($this->summaries);
    }

    /**
     * @return \ArrayIterator<int, NoteSummary>
     */
    public function getIterator(): \ArrayIterator
    {
        return new \ArrayIterator($this->summaries);
    }
}
