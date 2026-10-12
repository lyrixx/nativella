<?php

namespace Nativella;

/**
 * Fingerprint or face authentication, on phones: to unlock what only the
 * owner of the phone should see.
 *
 * The application asks for it with "biometrics" in the permissions of
 * extra.nativella, in its composer.json.
 */
final readonly class Biometrics
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Whether the phone has a fingerprint or a face enrolled.
     */
    public function isAvailable(): bool
    {
        return $this->client->call('biometrics.isAvailable')['available'];
    }

    /**
     * Asks the user to prove who they are, with their fingerprint, their
     * face or the PIN of the phone. Blocks until they do.
     *
     * @param string $reason what for, shown to the user
     *
     * @return bool false when the user cancels or does not match
     */
    public function authenticate(string $reason, ?string $title = null): bool
    {
        return $this->client->call('biometrics.authenticate', ['reason' => $reason, 'title' => $title])['authenticated'];
    }
}
