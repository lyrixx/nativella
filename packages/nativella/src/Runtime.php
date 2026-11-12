<?php

namespace Nativella;

/**
 * What the Nativella runtime tells PHP about the current request.
 *
 * The runtime sets these server variables itself, so a request coming from
 * a page cannot forge them. Its headers only ever become HTTP_* variables.
 */
final class Runtime
{
    /**
     * Where the runtime sends native events, as internal requests.
     *
     * Keep in sync with EVENT_PATH in crates/nativella/src/events.rs.
     */
    public const string EVENT_PATH = '/_nativella/event';

    private function __construct()
    {
    }

    /**
     * Whether PHP runs in Nativella. To offer native features, ask
     * Nativella::isNative() instead, which is also true under a FakeClient.
     */
    public static function isAvailable(): bool
    {
        return \function_exists('nativella_call');
    }

    /**
     * Null outside Nativella.
     */
    public static function version(): ?string
    {
        return self::server('NATIVELLA_VERSION');
    }

    /**
     * "linux", "macos", "windows", "android" or "ios". Null outside Nativella.
     */
    public static function platform(): ?string
    {
        return self::server('NATIVELLA_PLATFORM');
    }

    /**
     * True on a phone or a tablet, where there is no menu bar, tray icon,
     * global shortcut or autostart.
     */
    public static function isMobile(): bool
    {
        return \in_array(self::platform(), ['android', 'ios'], true);
    }

    /**
     * For the application's database and files, unique to the application and
     * the user. Null outside Nativella.
     */
    public static function dataDir(): ?string
    {
        return self::server('NATIVELLA_DATA_DIR');
    }

    /**
     * A secret unique to the installation (64 hexadecimal characters), for
     * the framework to sign and encrypt with. An application ships the same
     * code to every user, so it cannot ship its secret. Null outside Nativella.
     */
    public static function secret(): ?string
    {
        return self::server('NATIVELLA_SECRET');
    }

    /**
     * The label of the window whose page made the current request, null for
     * an internal request or outside Nativella.
     */
    public static function window(): ?string
    {
        return self::server('NATIVELLA_WINDOW');
    }

    /**
     * Whether the runtime itself sent the current request, to notify PHP of
     * a native event.
     */
    public static function isInternalRequest(): bool
    {
        return '1' === self::server('NATIVELLA_INTERNAL');
    }

    private static function server(string $name): ?string
    {
        $value = $_SERVER[$name] ?? null;

        return \is_string($value) && '' !== $value ? $value : null;
    }
}
