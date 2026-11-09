<?php

namespace Nativella\Event;

/**
 * The runtime started PHP, and waits for this event to be handled before
 * serving pages. Migrate the database, open windows and set the menus here. If
 * no window is open afterwards, the runtime opens the main one on "/".
 */
final readonly class AppBooted
{
}
