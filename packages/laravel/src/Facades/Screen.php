<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * The displays and the cursor.
 *
 * @method static list<\Nativella\Screen\Display> all()
 * @method static ?\Nativella\Screen\Display      primary()
 * @method static \Nativella\Screen\Point         cursorPosition()
 *
 * @see \Nativella\Screen
 */
final class Screen extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Screen::class;
    }
}
