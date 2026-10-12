<?php

namespace Nativella\Power;

enum BatteryState: string
{
    case Charging = 'charging';
    case Discharging = 'discharging';
    case Empty = 'empty';
    case Full = 'full';
    /** On mains, but held below full: a charge limit. */
    case NotCharging = 'notCharging';
    case Unknown = 'unknown';
}
