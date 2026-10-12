<?php

namespace Nativella\Symfony\Twig;

use Nativella\Runtime;
use Twig\Extension\AbstractExtension;
use Twig\TwigFunction;

/**
 * What templates need to know about the runtime:
 *
 * - nativella_is_native(): whether Nativella shows the page, to offer native
 *   features only there;
 * - nativella_is_mobile(): whether it runs on a phone, without menu bar, tray
 *   icon, global shortcut or file dropping;
 * - nativella_platform(): "linux", "macos", "windows", "android", "ios", or
 *   null outside Nativella.
 */
final class NativellaExtension extends AbstractExtension
{
    public function getFunctions(): array
    {
        return [
            new TwigFunction('nativella_is_mobile', Runtime::isMobile(...)),
            new TwigFunction('nativella_is_native', Runtime::isNative(...)),
            new TwigFunction('nativella_platform', Runtime::platform(...)),
        ];
    }
}
