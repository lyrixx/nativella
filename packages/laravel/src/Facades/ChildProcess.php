<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Support\Facades\Facade;

/**
 * Processes the application starts and watches, with their output as events.
 *
 * @method static int                                          start(string $alias, list<string> $command, ?string $cwd = null, array<string, string> $env = [])
 * @method static void                                         write(string $alias, string $data)
 * @method static void                                         closeInput(string $alias)
 * @method static void                                         stop(string $alias)
 * @method static bool                                         isRunning(string $alias)
 * @method static list<\Nativella\ChildProcess\RunningProcess> all()
 *
 * @see \Nativella\ChildProcess
 */
final class ChildProcess extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return \Nativella\ChildProcess::class;
    }
}
