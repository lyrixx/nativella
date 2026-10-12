<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Collection;

/**
 * @extends Collection<int, Task>
 */
final class TaskCollection extends Collection
{
    /**
     * The tasks as a Markdown checklist, for the clipboard and the export.
     */
    public function toMarkdown(): string
    {
        return $this->reduce(
            static fn (string $markdown, Task $task): string => $markdown.\sprintf("- [%s] %s\n", $task->isCompleted() ? 'x' : ' ', $task->title),
            "# Tasks\n\n",
        );
    }
}
