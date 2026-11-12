<?php

namespace Nativella\Client;

use Nativella\Exception\VersionMismatchException;

/**
 * @internal
 */
final readonly class VersionCheck
{
    private function __construct()
    {
    }

    /**
     * @param list<string> $packageVersions the version Composer installed, then its aliases ("dev-main", "0.5.x-dev")
     *
     * @throws VersionMismatchException when the two are not of the same release line
     */
    public static function ensureCompatible(array $packageVersions, string $runtimeVersion): void
    {
        $runtimeLine = self::releaseLine($runtimeVersion);
        if (null === $runtimeLine) {
            return;
        }

        foreach ($packageVersions as $packageVersion) {
            $packageLine = self::releaseLine($packageVersion);
            // A branch, which says nothing unless it has an alias.
            if (null === $packageLine) {
                continue;
            }
            if ($packageLine === $runtimeLine) {
                return;
            }
            if (version_compare($runtimeLine, $packageLine, '<')) {
                throw new VersionMismatchException(\sprintf('The nativella command (%s) is older than the nativella/nativella package (%s): upgrade the command to %s.', $runtimeVersion, $packageVersion, $packageLine));
            }

            throw new VersionMismatchException(\sprintf('The nativella/nativella package (%s) is older than the nativella command (%s): upgrade the package, and the integration that comes with it, to %s.', $packageVersion, $runtimeVersion, $runtimeLine));
        }
    }

    /**
     * "0.5" for 0.5.2 and 0.5.x-dev, "1" for 1.4.0: before 1.0, a minor version can break things.
     */
    private static function releaseLine(string $version): ?string
    {
        if (1 !== preg_match('/^v?(\d+)\.(\d+)/', $version, $matches)) {
            return null;
        }

        return '0' === $matches[1] ? '0.'.$matches[2] : $matches[1];
    }
}
