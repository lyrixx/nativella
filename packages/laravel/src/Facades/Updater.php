<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * A standalone executable updates itself.
 *
 * @method static ?\Nativella\Updater\Update check()
 * @method static ?string                    install()
 *
 * @see \Nativella\Updater
 */
final class Updater extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Updater::class;
    }
}
