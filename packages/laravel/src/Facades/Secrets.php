<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * @method static string|null get(string $key)
 * @method static void        set(string $key, string $value)
 * @method static void        delete(string $key)
 *
 * @see \Nativella\Secrets
 */
final class Secrets extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Secrets::class;
    }
}
