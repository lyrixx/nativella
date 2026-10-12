<?php

namespace Nativella;

/**
 * Hands URLs and files over to the applications of the system.
 */
final readonly class Shell
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Opens a URL in the default browser (or mail client, for mailto:).
     */
    public function openUrl(string $url): void
    {
        $this->client->call('shell.openUrl', ['url' => $url]);
    }

    /**
     * Opens a file or a directory with its default application.
     */
    public function openPath(string $path): void
    {
        $this->client->call('shell.openPath', ['path' => $path]);
    }

    /**
     * Moves a file or a directory to the trash, where the user can still
     * restore it (desktop only).
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
