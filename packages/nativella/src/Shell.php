<?php

namespace Nativella;

/**
 * Hands URLs and files over to the system's applications.
 */
final readonly class Shell
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * In the default browser (or mail client, for mailto:).
     */
    public function openUrl(string $url): void
    {
        $this->client->call('shell.openUrl', ['url' => $url]);
    }

    public function openPath(string $path): void
    {
        $this->client->call('shell.openPath', ['path' => $path]);
    }

    /**
     * Desktop only.
     */
    public function trash(string $path): void
    {
        $this->client->call('shell.trash', ['path' => $path]);
    }

    /**
     * Opens the file manager on the directory of $path, with $path selected.
     */
    public function showInFolder(string $path): void
    {
        $this->client->call('shell.showInFolder', ['path' => $path]);
    }
}
