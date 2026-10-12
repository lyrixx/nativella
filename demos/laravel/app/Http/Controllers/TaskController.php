<?php

namespace App\Http\Controllers;

use App\Models\Task;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\View\View;
use Nativella\Dialog;
use Nativella\Dialog\MessageKind;
use Nativella\Laravel\Facades\Nativella;

final readonly class TaskController
{
    public function index(): View
    {
        $tasks = Task::query()->ordered()->get();

        return view('tasks.index', [
            'openCount' => $tasks->reject->isCompleted()->count(),
            'tasks' => $tasks,
        ]);
    }

    public function store(Request $request): RedirectResponse
    {
        Task::create($request->validate([
            'title' => ['required', 'string', 'max:255'],
        ]));

        return to_route('tasks.index');
    }

    public function complete(Task $task): RedirectResponse
    {
        $task->complete();
        $task->save();

        return to_route('tasks.index');
    }

    public function reopen(Task $task): RedirectResponse
    {
        $task->reopen();
        $task->save();

        return to_route('tasks.index');
    }

    public function destroy(Task $task, Dialog $dialog): RedirectResponse
    {
        // In a browser, the page asks before sending the form.
        if (Nativella::isNative() && !$dialog->confirm(\sprintf('Delete "%s"?', $task->title), 'Delete the task', MessageKind::Warning, 'Delete', 'Cancel')) {
            return to_route('tasks.index');
        }

        $task->delete();

        return to_route('tasks.index')->with('status', \sprintf('"%s" deleted.', $task->title));
    }
}
