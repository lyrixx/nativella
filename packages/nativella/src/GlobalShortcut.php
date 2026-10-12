<?php

namespace Nativella;

/**
 * Keyboard shortcuts caught even when the application is in the background.
 * A press dispatches Event\GlobalShortcutPressed.
 *
 * Accelerators look like "CmdOrCtrl+Shift+K": modifiers (Shift, Control,
 * Alt, Super, CmdOrCtrl) and a key, joined by "+".
 */
final readonly class GlobalShortcut
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function register(string $accelerator): void
    {
        $this->client->call('globalShortcut.register', ['accelerator' => $accelerator]);
    }

    public function unregister(string $accelerator): void
    {
        $this->client->call('globalShortcut.unregister', ['accelerator' => $accelerator]);
    }

    public function unregisterAll(): void
    {
        $this->client->call('globalShortcut.unregisterAll');
    }

    public function isRegistered(string $accelerator): bool
    {
        return $this->client->call('globalShortcut.isRegistered', ['accelerator' => $accelerator])['registered'];
    }
}
