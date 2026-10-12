<?php

namespace Nativella\Haptics;

enum Feedback: string
{
    case Error = 'error';
    case Success = 'success';
    case Warning = 'warning';
}
