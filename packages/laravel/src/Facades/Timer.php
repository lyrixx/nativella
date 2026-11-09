<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * @method static void every(string $name, float $seconds)
 * @method static void after(string $name, float $seconds)
 * @method static void cancel(string $name)
 *
 * @see \Nativella\Timer
 */
final class Timer extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Timer::class;
    }
}
