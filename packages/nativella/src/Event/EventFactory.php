<?php

namespace Nativella\Event;

use Nativella\ChildProcess\Stream;
use Nativella\Exception\UnexpectedResponseException;
use Nativella\Runtime;
use Nativella\System\Theme;
use Nativella\Tray\MouseButton;

/**
 * Names and payloads: keep in sync with the callers of events::send() in
 * crates/nativella/src/.
 */
final class EventFactory
{
    private function __construct()
    {
    }

    /**
     * @param array<string, mixed> $payload
     */
    public static function create(string $name, array $payload): object
    {
        return match ($name) {
            'app.booted' => new AppBooted(),
            'app.reopened' => new AppReopened($payload['args'], $payload['cwd']),
            'app.urlsOpened' => new AppUrlsOpened($payload['urls']),
            'childProcess.exited' => new ChildProcessExited($payload['alias'], $payload['pid'], $payload['code']),
            'childProcess.output' => new ChildProcessOutput($payload['alias'], $payload['pid'], Stream::tryFrom($payload['stream']) ?? throw UnexpectedResponseException::unknownValue('stream', $payload['stream']), $payload['output']),
            'globalShortcut.pressed' => new GlobalShortcutPressed($payload['accelerator']),
            'menu.itemClicked' => new MenuItemClicked($payload['id'], $payload['checked']),
            'notification.clicked' => new NotificationClicked($payload['reference']),
            'power.resumed' => new PowerResumed(),
            'power.sourceChanged' => new PowerSourceChanged($payload['onBattery']),
            'timer.fired' => new TimerFired($payload['name']),
            'tray.clicked' => new TrayClicked(MouseButton::tryFrom($payload['button']) ?? throw UnexpectedResponseException::unknownValue('mouse button', $payload['button'])),
            'updater.failed' => new UpdaterFailed($payload['version'], $payload['message']),
            'updater.installed' => new UpdaterInstalled($payload['version']),
            'window.blurred' => new WindowBlurred($payload['window']),
            'window.closed' => new WindowClosed($payload['window']),
            'window.downloaded' => new WindowDownloaded($payload['window'], $payload['path']),
            'window.dragEntered' => new WindowDragEntered($payload['window'], $payload['paths']),
            'window.dragLeft' => new WindowDragLeft($payload['window']),
            'window.filesDropped' => new WindowFilesDropped($payload['window'], $payload['paths']),
            'window.focused' => new WindowFocused($payload['window']),
            'window.themeChanged' => new WindowThemeChanged($payload['window'], Theme::tryFrom($payload['theme']) ?? throw UnexpectedResponseException::unknownValue('theme', $payload['theme'])),
            default => new NativeEvent($name, $payload),
        };
    }

    /**
     * @param string $body the body of an internal request: {"name": "…", "payload": {…}}
     *
     * @throws UnexpectedResponseException when the body is not such a document
     */
    public static function fromJson(string $body): object
    {
        $event = self::decode($body);

        return self::create($event->name, $event->payload);
    }

    /**
     * What a framework dispatches: the event as its own class, when this
     * version of the package knows it, then as a NativeEvent, which every event
     * also comes as. A listener of a NativeEvent keeps hearing it once the
     * package gives the event a class.
     *
     * @param string $body the body of an internal request: {"name": "…", "payload": {…}}
     *
     * @return non-empty-list<object>
     *
     * @throws UnexpectedResponseException when the body is not such a document
     */
    public static function allFromJson(string $body): array
    {
        $native = self::decode($body);
        $event = self::create($native->name, $native->payload);

        return $event instanceof NativeEvent ? [$native] : [$event, $native];
    }

    /**
     * The event the current request carries, when the runtime sent it.
     *
     * For applications without a framework, at the top of the front controller:
     *
     *     if (null !== $event = EventFactory::fromGlobals()) {
     *         // handle $event
     *         http_response_code(204);
     *
     *         return;
     *     }
     */
    public static function fromGlobals(): ?object
    {
        $path = parse_url($_SERVER['REQUEST_URI'] ?? '/', \PHP_URL_PATH);
        if (!Runtime::isInternalRequest() || Runtime::EVENT_PATH !== $path || 'POST' !== ($_SERVER['REQUEST_METHOD'] ?? null)) {
            return null;
        }

        return self::fromJson((string) file_get_contents('php://input'));
    }

    private static function decode(string $body): NativeEvent
    {
        $data = json_decode($body, true);
        if (!\is_array($data) || !\is_string($data['name'] ?? null) || !\is_array($data['payload'] ?? null)) {
            throw new UnexpectedResponseException('A native event is a JSON object with a "name" string and a "payload" object.');
        }

        return new NativeEvent($data['name'], $data['payload']);
    }
}
