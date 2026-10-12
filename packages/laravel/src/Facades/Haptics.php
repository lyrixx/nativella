<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Vibrations and haptic feedback, on phones.
 *
 * @method static void vibrate(int $milliseconds = 300)
 * @method static void impact(\Nativella\Haptics\Impact $impact = \Nativella\Haptics\Impact::Medium)
 * @method static void feedback(\Nativella\Haptics\Feedback $feedback)
 * @method static void selection()
 *
 * @see \Nativella\Haptics
 */
final class Haptics extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Haptics::class;
    }
}
