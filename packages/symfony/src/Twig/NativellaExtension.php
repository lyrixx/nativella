<?php

namespace Nativella\Symfony\Twig;

use Nativella\Client;
use Nativella\Runtime;
use Twig\Extension\AbstractExtension;
use Twig\TwigFunction;

/**
 * What templates need to know about the runtime:
 *
 * - nativella_is_native(): whether native calls work, to offer native
 *   features only then: Nativella shows the page, or the tests use the fake
 *   client;
 * - nativella_is_mobile(): whether it runs on a phone, without menu bar, tray
 *   icon, global shortcut or file dropping;
 * - nativella_platform(): "linux", "macos", "windows", "android", "ios", or
 *   null outside Nativella.
 */
final class NativellaExtension extends AbstractExtension
{
    public function __construct(
        private readonly Client $client,
    ) {
    }

    public function getFunctions(): array
    {
        return [
            new TwigFunction('nativella_is_mobile', Runtime::isMobile(...)),
            new TwigFunction('nativella_is_native', $this->client->isNative(...)),
            new TwigFunction('nativella_platform', Runtime::platform(...)),
        ];
    }
}
