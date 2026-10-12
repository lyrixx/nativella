<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The application itself: its identity, its directories, its lifecycle. Not
 * Laravel's App facade: import this class, or alias it.
 *
 * @method static \Nativella\App\AppInfo info()
 * @method static string                 path(\Nativella\App\Directory $directory)
 * @method static void                   quit()
 * @method static void                   restart()
 * @method static void                   hide()
 * @method static void                   show()
 * @method static void                   setBadgeCount(?int $count, ?string $window = null)
 * @method static void                   enableAutostart()
 * @method static void                   disableAutostart()
 * @method static bool                   isAutostartEnabled()
 *
 * @see \Nativella\App
 */
final class App extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\App::class;
    }
}
