<?php

namespace App\Desktop;

use Nativella\Runtime;

/**
 * The global shortcuts of the application, caught even in the background.
 */
enum Shortcut: string
{
    case ShowWindow = 'CmdOrCtrl+Alt+Shift+T';

    /**
     * As the keyboard of this system names it: "Ctrl+Alt+Shift+T".
     */
    public function label(): string
    {
        return str_replace('CmdOrCtrl', 'macos' === Runtime::platform() ? 'Cmd' : 'Ctrl', $this->value);
    }
}
