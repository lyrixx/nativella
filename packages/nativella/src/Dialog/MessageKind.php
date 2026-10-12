<?php

namespace Nativella\Dialog;

enum MessageKind: string
{
    case Error = 'error';
    case Info = 'info';
    case Warning = 'warning';
}
