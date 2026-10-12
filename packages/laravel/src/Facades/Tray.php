<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * An icon in the system tray (the menu bar on macOS), with its menu.
 *
 * @method static void set(?string $tooltip = null, list<\Nativella\Menu\MenuItem> $menu = [], ?string $icon = null, ?string $title = null)
 * @method static void remove()
 *
 * @see \Nativella\Tray
 */
final class Tray extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Tray::class;
    }
}
