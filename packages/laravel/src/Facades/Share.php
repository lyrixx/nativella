<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The share sheet of the phone.
 *
 * @method static void text(string $text)
 * @method static void file(string $path, ?string $mimeType = null, ?string $title = null)
 *
 * @see \Nativella\Share
 */
final class Share extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Share::class;
    }
}
