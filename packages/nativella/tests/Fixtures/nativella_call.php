<?php

/**
 * What the runtime registers in PHP: loading this file makes PHP believe it
 * runs in Nativella. Calls are recorded in $GLOBALS['nativella_calls'] and
 * answered with an empty object, or fail with $GLOBALS['nativella_failure'] as
 * the runtime does: a RuntimeException.
 */
function nativella_call(string $method, string $json): string
{
    $GLOBALS['nativella_calls'][] = ['method' => $method, 'payload' => json_decode($json, true, flags: \JSON_THROW_ON_ERROR)];

    if (isset($GLOBALS['nativella_failure'])) {
        throw new RuntimeException($GLOBALS['nativella_failure']);
    }

    return '{}';
}
