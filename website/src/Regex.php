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

    /**
     * @param callable(array<int, string>): string $callback
     */
    public static function replaceCallback(string $pattern, callable $callback, string $subject): string
    {
        return preg_replace_callback($pattern, $callback, $subject) ?? throw self::error($pattern);
    }

    /**
     * @return array<int, string>|null the groups of the first match, if any
     */
    public static function match(string $pattern, string $subject): ?array
    {
        return match (preg_match($pattern, $subject, $matches)) {
            1 => $matches,
            0 => null,
            false => throw self::error($pattern),
        };
    }

    private static function error(string $pattern): \RuntimeException
    {
        return new \RuntimeException(\sprintf('The regular expression %s failed: %s.', $pattern, preg_last_error_msg()));
    }
}
