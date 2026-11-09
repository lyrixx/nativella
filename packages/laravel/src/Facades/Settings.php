<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * @method static mixed                get(string $key, mixed $default = null)
 * @method static bool                 has(string $key)
 * @method static void                 set(string $key, mixed $value)
 * @method static void                 forget(string $key)
 * @method static array<string, mixed> all()
 * @method static void                 clear()
 *
 * @see \Nativella\Settings
 */
final class Settings extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Settings::class;
    }
}
