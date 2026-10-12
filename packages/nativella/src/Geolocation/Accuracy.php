<?php

namespace Nativella\Geolocation;

enum Accuracy: string
{
    /** Wi-Fi and cell towers: fast and frugal, to within some tens of meters. */
    case Balanced = 'balanced';
    /** GPS if there is one: a few meters, at the cost of time and battery. */
    case High = 'high';
}
