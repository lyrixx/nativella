<x-layout title="Tasks">
    <form class="new-task" method="POST" action="{{ route('tasks.store') }}">
        @csrf
        <input type="text" name="title" value="{{ old('title') }}" placeholder="What needs to be done?" aria-label="New task" maxlength="255" required autofocus>
        <button type="submit" class="button primary">Add</button>
    </form>
    @error('title')
        <p class="field-error">{{ $message }}</p>
    @enderror

    @if ($tasks->isEmpty())
        <p class="empty">Nothing to do. Add a task above.</p>
    @else
        <ul class="tasks">
            @foreach ($tasks as $task)
                <li @class(['task', 'completed' => $task->isCompleted()])>
                    <form method="POST" action="{{ route($task->isCompleted() ? 'tasks.reopen' : 'tasks.complete', $task) }}">
                        @csrf
                        <button type="submit" class="check" aria-label="{{ $task->isCompleted() ? 'Reopen' : 'Complete' }} {{ $task->title }}"></button>
                    </form>
                    <span class="title">{{ $task->title }}</span>
                    <form method="POST" action="{{ route('tasks.destroy', $task) }}" @unlessnative onsubmit="return confirm('Delete this task?')" @endnative>
                        @csrf
                        @method('DELETE')
                        <button type="submit" class="delete" aria-label="Delete {{ $task->title }}" title="Delete">&times;</button>
                    </form>
                </li>
            @endforeach
        </ul>
    @endif

    <footer class="statusbar">
        <span class="count">{{ trans_choice(':count open task|:count open tasks', $openCount) }}</span>
        @native
            <form method="POST" action="{{ route('tasks.copy') }}">
                @csrf
                <button type="submit" class="button" @disabled($tasks->isEmpty())>Copy</button>
            </form>
            {{-- Phones have no dialog to save a file yet. --}}
            @unless (\Nativella\Runtime::isMobile())
                <form method="POST" action="{{ route('tasks.export') }}">
                    @csrf
                    <button type="submit" class="button" @disabled($tasks->isEmpty())>Export…</button>
                </form>
            @endunless
        @else
            <span class="hint">Copy, export and notifications come with the desktop application.</span>
        @endnative
    </footer>
</x-layout>
