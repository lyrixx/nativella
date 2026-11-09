<?php

namespace Nativella\Haptics;

enum Impact: string
{
    case Heavy = 'heavy';
    case Light = 'light';
    case Medium = 'medium';
    case Rigid = 'rigid';
    case Soft = 'soft';
}
