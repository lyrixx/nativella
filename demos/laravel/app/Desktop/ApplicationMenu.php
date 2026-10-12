<?php

namespace App\Desktop;

use Illuminate\Container\Attributes\Config;
use Nativella\Menu;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use Nativella\Runtime;

final readonly class ApplicationMenu
{
    public function __construct(
        private Menu $menu,
        #[Config('app.name')]
        private string $appName,
    ) {
    }

    public function set(): void
    {
        // Phones and tablets have no menu bar.
        if (Runtime::isMobile()) {
            return;
        }

        $macos = 'macos' === Runtime::platform();

        $this->menu->set(...array_filter([
            // The first submenu is the application menu on macOS: it has Quit.
            $macos ? MenuItem::appMenu($this->appName) : null,
            MenuItem::submenu(
                'File',
                MenuItem::link('tasks.new', 'New task', '/', 'CmdOrCtrl+N'),
                MenuItem::action(MenuAction::ExportTasks->value, 'Export…', 'CmdOrCtrl+E'),
                MenuItem::separator(),
                MenuItem::link('settings', 'Settings…', '/settings', 'CmdOrCtrl+,'),
                ...($macos ? [] : [MenuItem::separator(), MenuItem::role(Role::Quit)]),
            ),
            MenuItem::editMenu(),
            MenuItem::windowMenu(),
        ]));
    }
}
