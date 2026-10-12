<?php

namespace Nativella\Menu;

/**
 * Menu entries the system implements.
 *
 * Hide, HideOthers, Services and ShowAll only exist on macOS: other systems
 * leave them out. On Linux, where GTK implements few of them, Nativella does:
 * edit roles act on the focused page and keep the page's own shortcuts.
 */
enum Role: string
{
    case About = 'about';
    case CloseWindow = 'closeWindow';
    case Copy = 'copy';
    case Cut = 'cut';
    case Fullscreen = 'fullscreen';
    case Hide = 'hide';
    case HideOthers = 'hideOthers';
    case Maximize = 'maximize';
    case Minimize = 'minimize';
    case Paste = 'paste';
    case Quit = 'quit';
    case Redo = 'redo';
    case SelectAll = 'selectAll';
    case Services = 'services';
    case ShowAll = 'showAll';
    case Undo = 'undo';
}
