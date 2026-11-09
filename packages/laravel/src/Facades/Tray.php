<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * @method static void set(?string $tooltip = null, array<\Nativella\Menu\MenuItem> $menu = [], ?string $icon = null, ?string $title = null)
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
