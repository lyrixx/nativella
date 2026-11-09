<?php

namespace Nativella;

/**
 * Fingerprint or face authentication, on phones, to unlock what only the
 * phone's owner should see.
 *
 * The application asks for it with "biometrics" in
 * extra.nativella.permissions, in its composer.json.
 */
final readonly class Biometrics
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function isAvailable(): bool
    {
        return $this->client->call('biometrics.isAvailable')['available'];
    }

    /**
     * Blocks until the user proves who they are, with their fingerprint, their
     * face or the phone's PIN.
     *
     * Works without biometrics, with the PIN only, so do not skip it when
     * isAvailable() is false.
     *
     * @param string $reason what for, shown to the user
     *
     * @return bool false when the user cancels or does not match
     *
     * @throws Exception\NativeCallException when the phone has no lock at all
     */
    public function authenticate(string $reason, ?string $title = null): bool
    {
        return $this->client->call('biometrics.authenticate', ['reason' => $reason, 'title' => $title])['authenticated'];
    }
}
