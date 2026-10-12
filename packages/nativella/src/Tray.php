<?php

namespace Nativella;

use Nativella\Menu\MenuItem;

/**
 * An icon in the system tray (the menu bar on macOS), with its menu. While it
 * exists, closing the last window keeps the application running.
 *
 * A click dispatches Event\TrayClicked, except on Linux where only the menu
 * works.
 */
final readonly class Tray
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Creates the tray icon, or updates it.
     *
     * @param list<MenuItem> $menu
     * @param string|null    $icon  the path of a PNG file, the application icon when null
     * @param string|null    $title next to the icon, on macOS and Linux
     */
    public function set(?string $tooltip = null, array $menu = [], ?string $icon = null, ?string $title = null): void
    {
        $this->client->call('tray.set', [
            'tooltip' => $tooltip,
            'menu' => Menu::items($menu),
            'icon' => $icon,
            'title' => $title,
        ]);
    }

    public function remove(): void
    {
        $this->client->call('tray.remove');
    }
}
