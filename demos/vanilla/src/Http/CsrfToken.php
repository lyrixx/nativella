<?php

namespace App\Http;

/**
 * The token every form that posts sends back, kept in the PHP session. In a
 * browser (php -S), a page of another site could post these forms for the
 * user, but it cannot read the token. In Nativella, no other site reaches
 * PHP.
 */
final readonly class CsrfToken
{
    /** The form field that carries it. */
    public const string FIELD = '_token';

    private const string KEY = 'csrf_token';

    public function value(): string
    {
        Session::start();
        if (!\is_string($_SESSION[self::KEY] ?? null)) {
            $_SESSION[self::KEY] = bin2hex(random_bytes(32));
        }

        return $_SESSION[self::KEY];
    }

    /**
     * Without a session, a new token is drawn: nothing matches it.
     */
    public function isValid(Request $request): bool
    {
        return hash_equals($this->value(), $request->form(self::FIELD));
    }
}
