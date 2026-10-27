<?php

namespace Nativella;

use Nativella\Updater\Update;

/**
 * A standalone executable updates itself from the releases of the
 * application: see "updates" in extra.nativella. Desktop only.
 *
 * install() downloads in the background, then Event\UpdateInstalled tells that
 * the next start runs the new version, or Event\UpdateFailed why not.
 */
final readonly class Updater
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * The release of the endpoint, when it is newer than this version.
     */
    public function check(): ?Update
    {
        $update = $this->client->call('updater.check')['update'];

        return null === $update ? null : Update::fromArray($update);
    }

    /**
     * Starts installing the newer release, if there is one.
     *
     * @return string|null its version, null when this one is the latest
     *
     * @throws Exception\NativeCallException outside a standalone executable, or when the endpoint fails
     */
    public function install(): ?string
    {
        return $this->client->call('updater.install')['version'];
    }
}
