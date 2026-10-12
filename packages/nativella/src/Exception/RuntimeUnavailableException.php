<?php

namespace Nativella\Exception;

/**
 * PHP does not run in Nativella: in a browser through a regular web server,
 * or on the command line.
 */
final class RuntimeUnavailableException extends \RuntimeException implements ExceptionInterface
{
}
