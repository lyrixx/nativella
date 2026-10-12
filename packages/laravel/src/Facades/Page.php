<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Talks to the JavaScript of the pages.
 *
 * @method static void dispatch(string $event, array<string, mixed> $detail = [], ?string $window = null)
 *
 * @see \Nativella\Page
 */
final class Page extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Page::class;
    }
}
