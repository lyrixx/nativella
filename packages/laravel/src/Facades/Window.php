<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Opens and drives windows, each one known by its label.
 *
 * @method static \Nativella\Window\WindowInfo       open(string $label, ?\Nativella\Window\WindowOptions $options = null)
 * @method static void                               close(?string $window = null)
 * @method static list<\Nativella\Window\WindowInfo> all()
 * @method static \Nativella\Window\WindowInfo       get(?string $window = null)
 * @method static void                               setTitle(string $title, ?string $window = null)
 * @method static void                               setSize(float $width, float $height, ?string $window = null)
 * @method static void                               setPosition(float $x, float $y, ?string $window = null)
 * @method static void                               setAlwaysOnTop(bool $enabled, ?string $window = null)
 * @method static void                               setFullscreen(bool $enabled, ?string $window = null)
 * @method static void                               setResizable(bool $enabled, ?string $window = null)
 * @method static void                               setProgress(?int $percent, ?string $window = null)
 * @method static void                               center(?string $window = null)
 * @method static void                               minimize(?string $window = null)
 * @method static void                               maximize(?string $window = null)
 * @method static void                               unmaximize(?string $window = null)
 * @method static void                               show(?string $window = null)
 * @method static void                               hide(?string $window = null)
 * @method static void                               focus(?string $window = null)
 * @method static void                               reload(?string $window = null)
 * @method static void                               print(?string $window = null)
 * @method static void                               printToPdf(string $path, ?string $window = null)
 * @method static void                               navigate(string $url, ?string $window = null)
 * @method static void                               openDevtools(?string $window = null)
 *
 * @see \Nativella\Window
 */
final class Window extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Window::class;
    }
}
