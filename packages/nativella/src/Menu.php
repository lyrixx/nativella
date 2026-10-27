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
     * Changes the item `$id` in the application menu and the tray menu, without
     * setting them again: what is null stays as it is.
     */
    public function update(string $id, ?string $label = null, ?bool $enabled = null, ?bool $checked = null): void
    {
        $this->client->call('menu.update', array_filter(
            ['id' => $id, 'label' => $label, 'enabled' => $enabled, 'checked' => $checked],
            static fn (string|bool|null $value): bool => null !== $value,
        ));
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
