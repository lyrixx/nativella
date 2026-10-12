<?php

namespace Nativella\ChildProcess;

enum Stream: string
{
    case Stderr = 'stderr';
    case Stdout = 'stdout';
}
