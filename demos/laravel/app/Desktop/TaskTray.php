<?php

namespace App\Desktop;

use App\Models\Task;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use Nativella\Runtime;
use Nativella\Tray;

/**
 * The tray icon, whose tooltip counts the open tasks. It keeps the
 * application running when its window is closed. Phones and tablets have
 * none.
 */
final readonly class TaskTray
{
    public function __construct(
        private Tray $tray,
    ) {
    }

    public function refresh(): void
    {
        if (Runtime::isMobile()) {
            return;
        }

        $open = Task::query()->open()->count();

        $this->tray->set(
            tooltip: match ($open) {
                0 => 'Nothing to do',
                1 => '1 open task',
                default => "{$open} open tasks",
            },
            menu: [
                MenuItem::action(MenuAction::ShowWindow->value, 'Show Tasks'),
                MenuItem::separator(),
                MenuItem::role(Role::Quit),
            ],
        );
    }
}
