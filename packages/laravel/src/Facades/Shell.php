<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Hands URLs and files over to the applications of the system.
 *
 * @method static void openUrl(string $url)
 * @method static void openPath(string $path)
 * @method static void showInFolder(string $path)
 * @method static void trash(string $path)
 *
 * @see \Nativella\Shell
 */
final class Shell extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Shell::class;
    }
}
