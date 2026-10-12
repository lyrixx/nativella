<?php

namespace App\Http\Controllers;

use App\Models\Task;
use Illuminate\Http\RedirectResponse;
use Nativella\Clipboard;

final readonly class CopyTasksController
{
    public function __invoke(Clipboard $clipboard): RedirectResponse
    {
        $tasks = Task::query()->ordered()->get();
        $clipboard->writeText($tasks->toMarkdown());

        return to_route('tasks.index')->with('status', trans_choice(':count task copied to the clipboard.|:count tasks copied to the clipboard.', $tasks->count()));
    }
}
