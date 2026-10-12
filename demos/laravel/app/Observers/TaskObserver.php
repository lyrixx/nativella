<?php

namespace App\Observers;

use App\Desktop\TaskTray;
use App\Models\Task;
use Nativella\Haptics;
use Nativella\Haptics\Feedback;
use Nativella\Laravel\Facades\Nativella;
use Nativella\Notification;
use Nativella\Runtime;

/**
 * Keeps the desktop in step with the tasks: the tray counts the open ones,
 * a notification celebrates the last one done, and so does a vibration on
 * phones.
 */
final readonly class TaskObserver
{
    public function __construct(
        private Haptics $haptics,
        private Notification $notification,
        private TaskTray $tray,
    ) {
    }

    public function saved(Task $task): void
    {
        if (!Nativella::isNative()) {
            return;
        }

        $this->tray->refresh();

        if ($task->wasChanged('completed_at') && $task->isCompleted() && Task::query()->open()->doesntExist()) {
            $this->notification->send('All done!', 'Every task is completed. Time for a break.', reference: 'all-done');
            if (Runtime::isMobile()) {
                $this->haptics->feedback(Feedback::Success);
            }
        }
    }

    public function deleted(Task $task): void
    {
        if (Nativella::isNative()) {
            $this->tray->refresh();
        }
    }
}
