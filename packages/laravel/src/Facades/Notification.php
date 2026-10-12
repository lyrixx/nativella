<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Desktop notifications. Not Laravel's notifications: import this class, or
 * alias it.
 *
 * @method static void send(string $title, ?string $body = null, ?string $reference = null)
 *
 * @see \Nativella\Notification
 */
final class Notification extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Notification::class;
    }
}
