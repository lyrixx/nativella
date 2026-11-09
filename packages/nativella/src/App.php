<?php

namespace Nativella;

use Nativella\App\AppInfo;
use Nativella\App\Directory;

final readonly class App
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function info(): AppInfo
    {
        // @phpstan-ignore argument.type (the shape the runtime answers with: crates/nativella/src/native/app.rs)
        return AppInfo::fromArray($this->client->call('app.info'));
    }

    public function path(Directory $directory): string
    {
        return $this->client->call('app.path', ['directory' => $directory->value])['path'];
    }

    /**
     * The licenses of the software the application contains (PHP, its
     * libraries, Nativella's Rust crates…), which they ask to show or to ship
     * with it. Use it for an "about" page. Null when running from a
     * development build of Nativella, which has none.
     */
    public function thirdPartyNotices(): ?string
    {
        return $this->client->call('app.thirdPartyNotices')['text'];
    }

    /**
     * Quits once the current request is answered.
     */
    public function quit(): void
    {
        $this->client->call('app.quit');
    }

    /**
     * Restarts once the current request is answered.
     */
    public function restart(): void
    {
        $this->client->call('app.restart');
    }

    /**
     * macOS only.
     */
    public function hide(): void
    {
        $this->client->call('app.hide');
    }

    /**
     * Shows the windows App::hide() hid (macOS only).
     */
    public function show(): void
    {
        $this->client->call('app.show');
    }

    /**
     * On the application icon, in the dock or the taskbar (macOS and some
     * Linux desktops). Null removes it.
     */
    public function setBadgeCount(?int $count, ?string $window = null): void
    {
        $this->client->call('app.setBadgeCount', ['count' => $count, 'window' => $window]);
    }

    /**
     * Starts the application when the user logs in.
     */
    public function enableAutostart(): void
    {
        $this->client->call('app.enableAutostart');
    }

    public function disableAutostart(): void
    {
        $this->client->call('app.disableAutostart');
    }

    public function isAutostartEnabled(): bool
    {
        return $this->client->call('app.isAutostartEnabled')['enabled'];
    }
}
