<?php

namespace App\View;

/**
 * Messages for the next page, after a redirect, kept in the PHP session.
 *
 * Nativella keeps the cookies of the session for the webview: WebKit does not
 * for a custom scheme.
 */
final readonly class Flash
{
    private const string KEY = 'flash';

    public function add(string $message): void
    {
        self::startSession();
        $_SESSION[self::KEY][] = $message;
    }

    /**
     * @return list<string> the messages, which will not show again
     */
    public function consume(): array
    {
        // No cookie, no session: pages do not start one for nothing.
        if (\PHP_SESSION_ACTIVE !== session_status() && !isset($_COOKIE[session_name()])) {
            return [];
        }

        self::startSession();
        $messages = $_SESSION[self::KEY] ?? [];
        unset($_SESSION[self::KEY]);

        return $messages;
    }

    private static function startSession(): void
    {
        if (\PHP_SESSION_ACTIVE !== session_status()) {
            session_start(['cookie_httponly' => true, 'cookie_samesite' => 'Lax', 'use_strict_mode' => true]);
        }
    }
}
