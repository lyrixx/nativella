<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Fingerprint or face authentication, on phones.
 *
 * @method static bool isAvailable()
 * @method static bool authenticate(string $reason, ?string $title = null)
 *
 * @see \Nativella\Biometrics
 */
final class Biometrics extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Biometrics::class;
    }
}
