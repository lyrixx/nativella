<?php

namespace App\Http;

/**
 * The PHP session, for the flash messages and the CSRF token.
 *
 * Nativella keeps the cookies of the session for the webview: WebKit does not
 * for a custom scheme.
 */
final class Session
{
    private function __construct()
    {
    }

    public static function start(): void
    {
        if (\PHP_SESSION_ACTIVE !== session_status()) {
            session_start(['cookie_httponly' => true, 'cookie_samesite' => 'Lax', 'use_strict_mode' => true]);
        }
    }

    /**
     * Whether a session is started, or the browser holds the cookie of one.
     */
    public static function exists(): bool
    {
        return \PHP_SESSION_ACTIVE === session_status() || isset($_COOKIE[session_name()]);
    }
}
