<?php

namespace App\Native;

use App\Import\ImportResult;
use App\Import\SnippetImporter;
use App\Snippet\SnippetSchema;
use Nativella\Event\AppBooted;
use Nativella\Event\GlobalShortcutPressed;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\WindowFilesDropped;
use Nativella\Exception\NativeCallException;
use Nativella\Menu\MenuItem;
use Nativella\Menu\Role;
use Nativella\Nativella;
use Nativella\Runtime;

/**
 * Handles the native events the runtime sends to PHP as internal requests.
 */
final readonly class NativeEventHandler
{
    /** Brings the window to the front, from any application. */
    public const string FOCUS_SHORTCUT = 'CmdOrCtrl+Shift+Space';

    /**
     * Tells the pages that the list changed. Keep in sync with public/app.js.
     */
    public const string IMPORTED_EVENT = 'snippets.imported';

    public const string MENU_IMPORT = 'import';
    public const string MENU_NEW_SNIPPET = 'new-snippet';
    public const string MENU_SYSTEM = 'system';

    public function __construct(
        private SnippetImporter $importer,
        private Nativella $native,
        private SnippetSchema $schema,
    ) {
    }

    public function handle(object $event): void
    {
        if ($event instanceof AppBooted) {
            $this->boot();
        } elseif ($event instanceof GlobalShortcutPressed && self::FOCUS_SHORTCUT === $event->accelerator) {
            $this->native->window->focus();
        } elseif ($event instanceof MenuItemClicked && self::MENU_IMPORT === $event->id) {
            $this->importChosenFile();
        } elseif ($event instanceof WindowFilesDropped) {
            $this->announce($this->importer->importFiles($event->paths));
        }
    }

    /**
     * The runtime serves no page before this returns.
     */
    private function boot(): void
    {
        $this->schema->migrate();

        // Phones and tablets have no menu bar, nor a keyboard for shortcuts.
        if (Runtime::isMobile()) {
            return;
        }

        $this->native->menu->set(...$this->menu());

        try {
            $this->native->globalShortcut->register(self::FOCUS_SHORTCUT);
        } catch (NativeCallException $e) {
            // Another application may hold the shortcut: the app works without it.
            error_log(\sprintf('Warning: the global shortcut %s is not available. %s', self::FOCUS_SHORTCUT, $e->getMessage()));
        }
    }

    /**
     * @return list<MenuItem>
     */
    private function menu(): array
    {
        $file = [
            MenuItem::link(self::MENU_NEW_SNIPPET, 'New snippet', '/snippets/new', 'CmdOrCtrl+N'),
            MenuItem::action(self::MENU_IMPORT, 'Import…', 'CmdOrCtrl+O'),
        ];

        // On macOS, the first submenu is named after the application, and
        // holds Quit; elsewhere, Quit goes at the end of File.
        if ('macos' === Runtime::platform()) {
            $menu = [MenuItem::appMenu($this->native->app->info()->name)];
        } else {
            $menu = [];
            $file[] = MenuItem::separator();
            $file[] = MenuItem::role(Role::Quit);
        }

        return [
            ...$menu,
            MenuItem::submenu('File', ...$file),
            MenuItem::editMenu(),
            MenuItem::windowMenu(),
            MenuItem::submenu('Help', MenuItem::link(self::MENU_SYSTEM, 'System information', '/system')),
        ];
    }

    private function importChosenFile(): void
    {
        $result = $this->importer->importChosenFile();
        if (null !== $result) {
            $this->announce($result);
        }
    }

    /**
     * No page reloads after a drop or a menu click: the pages refresh their
     * list themselves.
     */
    private function announce(ImportResult $result): void
    {
        $this->native->page->dispatch(self::IMPORTED_EVENT, ['message' => $result->message()]);
    }
}
