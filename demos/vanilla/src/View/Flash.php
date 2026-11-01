<?php

namespace App\View;

use App\Http\Session;

/**
 * Messages for the next page, after a redirect, kept in the PHP session.
 */
final readonly class Flash
{
    private const string KEY = 'flash';

    public function add(string $message): void
    {
        Session::start();
        $_SESSION[self::KEY][] = $message;
    }

    /**
     * @return list<string> the messages, which will not show again
     */
    public function consume(): array
    {
        // No cookie, no session: pages do not start one for nothing.
        if (!Session::exists()) {
            return [];
        }

        Session::start();
        $messages = $_SESSION[self::KEY] ?? [];
        unset($_SESSION[self::KEY]);

        return $messages;
    }
}
