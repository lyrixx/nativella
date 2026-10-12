<?php

namespace Nativella\Event;

/**
 * The runtime started PHP, and waits for this event to be handled before
 * serving pages: migrate the database, open windows, set the menus here. If
 * no window is open afterwards, the runtime opens the main one on "/".
 */
final readonly class AppBooted
{
}
