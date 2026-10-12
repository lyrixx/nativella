<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The system clipboard.
 *
 * @method static ?string readText()
 * @method static void    writeText(string $text)
 * @method static void    writeHtml(string $html, ?string $altText = null)
 * @method static ?string readImage()
 * @method static void    writeImage(string $png)
 * @method static void    clear()
 *
 * @see \Nativella\Clipboard
 */
final class Clipboard extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Clipboard::class;
    }
}
