<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The application menu, and context menus.
 *
 * @method static void set(\Nativella\Menu\MenuItem ...$items)
 * @method static void remove()
 * @method static void popup(\Nativella\Menu\MenuItem ...$items)
 *
 * @see \Nativella\Menu
 */
final class Menu extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Menu::class;
    }
}
