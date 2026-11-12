<?php

namespace Nativella\Exception;

/**
 * The runtime, or the endpoint of the updates, answered something the package
 * cannot read: bytes that are not base64, a date that is none, a value the
 * package does not know.
 */
final class UnexpectedResponseException extends \UnexpectedValueException implements ExceptionInterface
{
    public static function unknownValue(string $what, string $value): self
    {
        return new self(\sprintf('The runtime sent a %s the package does not know: "%s".', $what, $value));
    }
}
