<?php

namespace Nativella;

use Nativella\Menu\MenuItem;

/**
 * The application menu, and context menus.
 *
 *     $menu->set(
 *         MenuItem::appMenu('Notes'),
 *         MenuItem::submenu('File', MenuItem::link('new', 'New note', '/notes/new', 'CmdOrCtrl+N')),
 *         MenuItem::editMenu(),
 *     );
 *
 * On macOS, the top level only holds submenus.
 */
final readonly class Menu
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function set(MenuItem ...$items): void
    {
        $this->client->call('menu.set', ['items' => self::items($items)]);
    }

    public function remove(): void
    {
        $this->client->call('menu.remove');
    }

    /**
     * Opens a context menu at the mouse cursor, in the window that made the
     * request.
     */
    public function popup(MenuItem ...$items): void
    {
        $this->client->call('menu.popup', ['items' => self::items($items)]);
    }

    /**
     * @param array<MenuItem> $items
     *
     * @return list<array<string, mixed>>
     */
    public static function items(array $items): array
    {
        return array_values(array_map(static fn (MenuItem $item): array => $item->toArray(), $items));
    }
}
