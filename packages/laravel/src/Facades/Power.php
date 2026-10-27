<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The battery and the power source of the computer, and keeping it awake.
 *
 * @method static bool                      isOnBattery()
 * @method static ?\Nativella\Power\Battery battery()
 * @method static void                      keepAwake(string $reason)
 * @method static void                      allowSleep()
 *
 * @see \Nativella\Power
 */
final class Power extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Power::class;
    }
}
