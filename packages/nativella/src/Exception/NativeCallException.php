<?php

namespace Nativella\Exception;

/**
 * The native side could not do what PHP asked: the window does not exist,
 * the payload is invalid (a string that is not UTF-8, INF or NAN cannot even
 * be sent), or the system refused.
 */
final class NativeCallException extends \RuntimeException implements ExceptionInterface
{
    public function __construct(
        public readonly string $method,
        \RuntimeException|\JsonException $previous,
    ) {
        parent::__construct(\sprintf('The native call "%s" failed: %s', $method, $previous->getMessage()), 0, $previous);
    }
}
