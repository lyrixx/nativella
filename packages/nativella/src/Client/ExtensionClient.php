<?php

namespace Nativella\Client;

use Composer\InstalledVersions;
use Nativella\Client;
use Nativella\Exception\NativeCallException;
use Nativella\Exception\RuntimeUnavailableException;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\Runtime;

/**
 * Calls the runtime through the nativella_call() function it registers in PHP.
 */
final readonly class ExtensionClient implements Client
{
    private const string PACKAGE = 'nativella/nativella';

    public function call(string $method, array $payload = []): array
    {
        if (!Runtime::isAvailable()) {
            throw new RuntimeUnavailableException(\sprintf('Cannot call "%s": the application does not run in Nativella.', $method));
        }

        self::ensureSameReleaseLine();

        try {
            $json = json_encode((object) $payload, \JSON_THROW_ON_ERROR | \JSON_PRESERVE_ZERO_FRACTION | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE);
        } catch (\JsonException $e) {
            // A string that is not UTF-8: text read from a file in another encoding.
            throw new NativeCallException($method, $e);
        }

        try {
            $result = nativella_call($method, $json);
        } catch (\RuntimeException $e) {
            throw new NativeCallException($method, $e);
        }

        $decoded = json_decode($result, true);
        if (!\is_array($decoded)) {
            throw new UnexpectedResponseException(\sprintf('The runtime answered "%s" with something that is not a JSON object.', $method));
        }

        return $decoded;
    }

    public function isNative(): bool
    {
        return Runtime::isAvailable();
    }

    /**
     * Checked on the first call only: neither version changes while PHP runs.
     */
    private static function ensureSameReleaseLine(): void
    {
        static $same = false;
        $runtimeVersion = Runtime::version();
        if ($same || null === $runtimeVersion || !InstalledVersions::isInstalled(self::PACKAGE)) {
            return;
        }

        $versions = [];
        foreach (InstalledVersions::getAllRawData() as $installed) {
            $package = $installed['versions'][self::PACKAGE] ?? [];
            if (isset($package['pretty_version'])) {
                $versions = [$package['pretty_version'], ...array_values($package['aliases'] ?? [])];

                break;
            }
        }

        VersionCheck::ensureCompatible($versions, $runtimeVersion);
        $same = true;
    }
}
