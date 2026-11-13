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
     * @param array<MenuItem> $menu   a list, or what array_filter() leaves of one
     * @param string|null     $icon   the path of a PNG file, the application icon when null
     * @param string|null     $title  next to the icon, on macOS and Linux
     * @param string|null     $window the label of a window, opened hidden, that a left click on the icon shows next to it, and hides
     */
    public function set(?string $tooltip = null, array $menu = [], ?string $icon = null, ?string $title = null, ?string $window = null): void
    {
        $this->client->call('tray.set', [
            'tooltip' => $tooltip,
            // A JSON array, whatever the keys.
            'menu' => array_map(static fn (MenuItem $item): array => $item->toArray(), array_values($menu)),
            'icon' => $icon,
            'title' => $title,
            'window' => $window,
        ]);
    }

    /**
     * Shows the window of the tray icon and focuses it: next to the icon on
     * macOS and Windows, in the corner of the screen on Linux, which does not
     * tell where the icon is.
     */
    public function showWindow(): void
    {
        $this->client->call('tray.showWindow');
    }

    public function remove(): void
    {
        $this->client->call('tray.remove');
    }
}
