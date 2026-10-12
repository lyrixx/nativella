<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The operating system and its preferences.
 *
 * @method static \Nativella\System\SystemInfo info()
 * @method static \Nativella\System\Theme      theme(?string $window = null)
 *
 * @see \Nativella\System
 */
final class System extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\System::class;
    }
}
