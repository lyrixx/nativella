<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Where the phone is.
 *
 * @method static ?\Nativella\Geolocation\Position currentPosition(\Nativella\Geolocation\Accuracy $accuracy = \Nativella\Geolocation\Accuracy::Balanced, int $maximumAge = 0)
 *
 * @see \Nativella\Geolocation
 */
final class Geolocation extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Geolocation::class;
    }
}
