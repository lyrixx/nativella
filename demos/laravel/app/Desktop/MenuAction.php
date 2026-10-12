<?php

namespace App\Desktop;

/**
 * The menu items PHP handles, by id. Links ("New task"…) and system roles
 * (Quit, the Edit and Window menus) need no handling: the runtime does it.
 */
enum MenuAction: string
{
    case ExportTasks = 'tasks.export';
    case ShowWindow = 'window.show';
}
