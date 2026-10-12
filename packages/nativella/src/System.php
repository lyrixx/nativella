<?php

namespace Nativella;

use Nativella\System\SystemInfo;
use Nativella\System\Theme;

/**
 * The operating system and its preferences.
 */
final readonly class System
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function info(): SystemInfo
    {
        // @phpstan-ignore argument.type (the shape the runtime answers with: crates/nativella/src/native/system.rs)
        return SystemInfo::fromArray($this->client->call('system.info'));
    }

    /**
     * The theme of a window, which follows the system one.
     */
    public function theme(?string $window = null): Theme
    {
        return Theme::from($this->client->call('system.theme', ['window' => $window])['theme']);
    }
}
