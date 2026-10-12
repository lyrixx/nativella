<?php

namespace App\Tasks;

use App\Models\Task;
use Nativella\Dialog;
use Nativella\Dialog\FileFilter;

final readonly class TaskExporter
{
    public function __construct(
        private Dialog $dialog,
    ) {
    }

    /**
     * Asks where to save the tasks, then writes them there as Markdown.
     *
     * @return string|null the file written, null if the user cancelled
     *
     * @throws ExportFailed when the file cannot be written
     */
    public function export(): ?string
    {
        $path = $this->dialog->saveFile('Export the tasks', fileName: 'tasks.md', filters: [new FileFilter('Markdown', ['md'])]);
        if (null === $path) {
            return null;
        }

        try {
            file_put_contents($path, Task::query()->ordered()->get()->toMarkdown());
        } catch (\ErrorException $e) {
            // Laravel turns the warning of a failed write into an exception.
            throw new ExportFailed($path, $e);
        }

        return $path;
    }
}
