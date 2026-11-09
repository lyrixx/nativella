<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * @method static bool               isAvailable()
 * @method static \Nativella\Nfc\Tag read()
 *
 * @see \Nativella\Nfc
 */
final class Nfc extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Nfc::class;
    }
}
