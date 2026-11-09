<?php

namespace Nativella;

use Nativella\System\SystemInfo;
use Nativella\System\Theme;

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

    public function theme(?string $window = null): Theme
    {
        return Theme::from($this->client->call('system.theme', ['window' => $window])['theme']);
    }
}
