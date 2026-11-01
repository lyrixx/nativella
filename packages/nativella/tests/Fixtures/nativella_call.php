<?php

/**
 * What the runtime registers in PHP: loading this file makes PHP believe it
 * runs in Nativella. Calls are recorded in $GLOBALS['nativella_calls'] and
 * answered with an empty object.
 */
function nativella_call(string $method, string $json): string
{
    $GLOBALS['nativella_calls'][] = ['method' => $method, 'payload' => json_decode($json, true, flags: \JSON_THROW_ON_ERROR)];

    return '{}';
}
