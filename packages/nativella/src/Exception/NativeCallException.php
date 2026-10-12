<?php

namespace Nativella\Exception;

/**
 * The native side could not do what PHP asked: the window does not exist,
 * the payload is invalid, or the system refused.
 */
final class NativeCallException extends \RuntimeException implements ExceptionInterface
{
    public function __construct(
        public readonly string $method,
        \RuntimeException $previous,
    ) {
        parent::__construct(\sprintf('The native call "%s" failed: %s', $method, $previous->getMessage()), 0, $previous);
    }
}
