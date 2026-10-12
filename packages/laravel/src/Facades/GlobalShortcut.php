<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Keyboard shortcuts caught even when the application is in the background.
 *
 * @method static void register(string $accelerator)
 * @method static void unregister(string $accelerator)
 * @method static void unregisterAll()
 * @method static bool isRegistered(string $accelerator)
 *
 * @see \Nativella\GlobalShortcut
 */
final class GlobalShortcut extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\GlobalShortcut::class;
    }
}
