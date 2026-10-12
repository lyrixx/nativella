<?php

namespace Nativella\Website;

/**
 * preg_* functions that throw instead of returning null or false.
 */
final class Regex
{
    private function __construct()
    {
    }

    public static function replace(string $pattern, string $replacement, string $subject): string
    {
        return preg_replace($pattern, $replacement, $subject) ?? throw self::error($pattern);
    }

    private static function error(string $pattern): \RuntimeException
    {
        return new \RuntimeException(\sprintf('The regular expression %s failed: %s.', $pattern, preg_last_error_msg()));
    }
}
