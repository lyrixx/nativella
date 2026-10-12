<?php

namespace Nativella\Haptics;

/**
 * How hard a tap feels.
 */
enum Impact: string
{
    case Heavy = 'heavy';
    case Light = 'light';
    case Medium = 'medium';
    case Rigid = 'rigid';
    case Soft = 'soft';
}
