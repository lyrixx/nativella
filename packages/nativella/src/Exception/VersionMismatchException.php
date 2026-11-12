<?php

namespace Nativella\Exception;

/**
 * The nativella command and the nativella/nativella package are released
 * together, and only talk to each other within a release line: 0.5.x with
 * 0.5.x before 1.0, 1.x with 1.x after.
 */
final class VersionMismatchException extends \RuntimeException implements ExceptionInterface
{
}
