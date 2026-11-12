<?php

namespace Nativella;

use Nativella\Exception\UnexpectedResponseException;
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

    /**
     * The theme of the window. While the application has no window, as on
     * Event\AppBooted, the theme of the system on Linux, and light elsewhere.
     */
    public function theme(?string $window = null): Theme
    {
        $theme = $this->client->call('system.theme', ['window' => $window])['theme'];

        return Theme::tryFrom($theme) ?? throw UnexpectedResponseException::unknownValue('theme', $theme);
    }
}
