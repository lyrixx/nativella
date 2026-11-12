<?php

namespace Nativella\Client;

use Composer\InstalledVersions;
use Nativella\Client;
use Nativella\Exception\ErrorKind;
use Nativella\Exception\NativeCallException;
use Nativella\Exception\RuntimeUnavailableException;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\Runtime;

/**
 * Calls the runtime through the nativella_call() function it registers in PHP,
 * which answers {"result": {…}} or {"error": {"kind": "…", "message": "…"}}.
 * Keep in sync with answer() in crates/runtime/src/bridge.rs.
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

        $json = Payload::encode($method, $payload);

        $answer = json_decode(nativella_call($method, $json), true);
        if (\is_array($answer) && \is_array($answer['result'] ?? null)) {
            return $answer['result'];
        }

        $error = \is_array($answer) ? $answer['error'] ?? null : null;
        if (!\is_array($error) || !\is_string($error['kind'] ?? null) || !\is_string($error['message'] ?? null)) {
            throw new UnexpectedResponseException(\sprintf('The runtime answered "%s" with neither a result nor an error.', $method));
        }

        throw new NativeCallException($method, ErrorKind::tryFrom($error['kind']) ?? throw UnexpectedResponseException::unknownValue('kind of error', $error['kind']), $error['message']);
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
