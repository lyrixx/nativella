<?php

namespace App\Native;

use App\Repository\NoteRepository;
use Nativella\Menu;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use Nativella\Settings;
use Nativella\Tray;
use Nativella\Window;
use Nativella\Window\WindowOptions;

/**
 * The native parts of the application: its window, menu and tray icon.
 */
final readonly class Desktop
{
    public const string MAIN_WINDOW = 'main';
    public const string QUICK_NOTE_SHORTCUT = 'CmdOrCtrl+Shift+N';
    public const string SETTING_ALWAYS_ON_TOP = 'alwaysOnTop';

    public function __construct(
        private Menu $menu,
        private NoteRepository $notes,
        private Settings $settings,
        private Tray $tray,
        private Window $window,
    ) {
    }

    public function openMainWindow(): void
    {
        $this->window->open(self::MAIN_WINDOW, new WindowOptions(
            title: 'Notes',
            width: 1100,
            height: 720,
            minWidth: 640,
            minHeight: 420,
            alwaysOnTop: $this->isAlwaysOnTop(),
        ));
    }

    public function installMenu(): void
    {
        $this->menu->set(
            MenuItem::appMenu('Nativella Notes'),
            MenuItem::submenu(
                'File',
                MenuItem::link('file.new', 'New note', '/notes/new', 'CmdOrCtrl+N'),
                MenuItem::action('file.import', 'Import…', 'CmdOrCtrl+O'),
                MenuItem::separator(),
                MenuItem::link('file.settings', 'Settings', '/settings', 'CmdOrCtrl+,'),
            ),
            MenuItem::editMenu(),
            MenuItem::submenu(
                'View',
                MenuItem::checkbox('view.alwaysOnTop', 'Always on top', $this->isAlwaysOnTop()),
                MenuItem::separator(),
                MenuItem::action('view.reload', 'Reload', 'CmdOrCtrl+R'),
                MenuItem::action('view.devtools', 'Developer tools', 'CmdOrCtrl+Alt+I'),
            ),
            MenuItem::windowMenu(),
            MenuItem::submenu(
                'Help',
                MenuItem::action('help.website', 'Nativella on GitHub'),
            ),
        );
    }

    /**
     * The tray icon shows how many notes there are.
     */
    public function refreshTray(): void
    {
        $count = $this->notes->count([]);

        $this->tray->set(
            tooltip: \sprintf('Nativella Notes: %d %s', $count, 1 === $count ? 'note' : 'notes'),
            menu: [
                MenuItem::action('tray.show', 'Show Notes'),
                MenuItem::link('tray.new', 'New note', '/notes/new'),
                MenuItem::separator(),
                MenuItem::role(Role::Quit),
            ],
        );
    }

    public function isAlwaysOnTop(): bool
    {
        return true === $this->settings->get(self::SETTING_ALWAYS_ON_TOP, false);
    }

    public function setAlwaysOnTop(bool $enabled): void
    {
        $this->settings->set(self::SETTING_ALWAYS_ON_TOP, $enabled);
        $this->window->setAlwaysOnTop($enabled, self::MAIN_WINDOW);
        // The checkbox of the menu follows.
        $this->installMenu();
    }

    public function showMainWindow(): void
    {
        $this->openMainWindow();
        $this->window->show(self::MAIN_WINDOW);
        $this->window->focus(self::MAIN_WINDOW);
    }
}
