<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Message boxes and file pickers, modal to the window that made the request.
 *
 * @method static void         message(string $message, ?string $title = null, \Nativella\Dialog\MessageKind $kind = \Nativella\Dialog\MessageKind::Info)
 * @method static bool         confirm(string $message, ?string $title = null, \Nativella\Dialog\MessageKind $kind = \Nativella\Dialog\MessageKind::Info, ?string $okLabel = null, ?string $cancelLabel = null)
 * @method static bool         ask(string $message, ?string $title = null, \Nativella\Dialog\MessageKind $kind = \Nativella\Dialog\MessageKind::Info)
 * @method static ?string      openFile(?string $title = null, ?string $directory = null, array<\Nativella\Dialog\FileFilter> $filters = [])
 * @method static list<string> openFiles(?string $title = null, ?string $directory = null, array<\Nativella\Dialog\FileFilter> $filters = [])
 * @method static ?string      openDirectory(?string $title = null, ?string $directory = null)
 * @method static ?string      saveFile(?string $title = null, ?string $directory = null, ?string $fileName = null, array<\Nativella\Dialog\FileFilter> $filters = [])
 *
 * @see \Nativella\Dialog
 */
final class Dialog extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Dialog::class;
    }
}
