<?php

namespace Nativella\Client;

use Nativella\Client;
use Nativella\Exception\NativeCallException;

/**
 * Records the calls and answers what the native side would when nothing
 * happens (no file picked, an empty clipboard…), or what the test told it to.
 */
final class FakeClient implements Client
{
    /**
     * What the runtime answers by default. Keep in sync with
     * crates/nativella/src/native/.
     */
    private const array DEFAULTS = [
        'app.info' => ['name' => 'Nativella', 'version' => '0.1.0', 'identifier' => 'dev.nativella.app', 'nativellaVersion' => '0.1.0'],
        'app.isAutostartEnabled' => ['enabled' => false],
        'app.thirdPartyNotices' => ['text' => null],
        'barcodeScanner.scan' => ['barcode' => null],
        'biometrics.authenticate' => ['authenticated' => false],
        'biometrics.isAvailable' => ['available' => false],
        'childProcess.all' => ['processes' => []],
        'childProcess.start' => ['pid' => 1],
        'clipboard.readImage' => ['png' => null],
        'clipboard.readText' => ['text' => null],
        'dialog.ask' => ['answer' => false],
        'dialog.confirm' => ['confirmed' => false],
        'dialog.openDirectory' => ['path' => null],
        'dialog.openFile' => ['path' => null],
        'dialog.openFiles' => ['paths' => []],
        'dialog.saveFile' => ['path' => null],
        'geolocation.currentPosition' => ['position' => null],
        'globalShortcut.isRegistered' => ['registered' => false],
        'nfc.isAvailable' => ['available' => false],
        'power.status' => ['onBattery' => false, 'battery' => null],
        'screen.all' => ['displays' => []],
        'screen.cursorPosition' => ['x' => 0, 'y' => 0],
        'screen.primary' => ['display' => null],
        'secrets.get' => ['value' => null],
        'settings.all' => ['settings' => []],
        'settings.has' => ['has' => false],
        'system.info' => ['os' => 'linux', 'family' => 'unix', 'arch' => 'x86_64', 'version' => '1.0.0', 'hostname' => 'localhost', 'locale' => 'en-US'],
        'system.theme' => ['theme' => 'light'],
        'updater.check' => ['update' => null],
        'updater.install' => ['version' => null],
        'window.all' => ['windows' => []],
    ];

    /** @var list<Call> */
    private array $calls = [];

    /** @var array<string, list<array<string, mixed>|\Throwable>> */
    private array $responses = [];

    /**
     * Answers the next call to $method with $result, or throws it.
     *
     * @param array<string, mixed>|\Throwable $result
     */
    public function respond(string $method, array|\Throwable $result): void
    {
        $this->responses[$method][] = $result;
    }

    public function call(string $method, array $payload = []): array
    {
        $this->calls[] = new Call($method, $payload);

        if ([] !== ($this->responses[$method] ?? [])) {
            $result = array_shift($this->responses[$method]);
            if ($result instanceof \Throwable) {
                throw $result;
            }

            return $result;
        }

        if (\in_array($method, ['window.open', 'window.get'], true)) {
            return ['window' => self::window($payload['window'] ?? 'main')];
        }
        if ('app.path' === $method) {
            return ['path' => '/home/nativella/'.$payload['directory']];
        }
        if ('nfc.read' === $method) {
            // What the runtime does when NFC is not available, as nfc.isAvailable answers.
            throw new NativeCallException($method, new \RuntimeException('NFC is not available: the phone has none, or it is turned off in its settings.'));
        }

        return self::DEFAULTS[$method] ?? [];
    }

    /**
     * Always true, so the code under test takes its native branches.
     */
    public function isNative(): bool
    {
        return true;
    }

    /**
     * @return list<Call>
     */
    public function calls(?string $method = null): array
    {
        if (null === $method) {
            return $this->calls;
        }

        return array_values(array_filter($this->calls, static fn (Call $call): bool => $call->method === $method));
    }

    public function reset(): void
    {
        $this->calls = [];
        $this->responses = [];
    }

    /**
     * @return array<string, mixed>
     */
    private static function window(string $window): array
    {
        return [
            'window' => $window,
            'title' => 'Nativella',
            'url' => 'php://localhost/',
            'width' => 1024.0,
            'height' => 768.0,
            'x' => 0.0,
            'y' => 0.0,
            'focused' => true,
            'maximized' => false,
            'minimized' => false,
            'fullscreen' => false,
            'visible' => true,
        ];
    }
}
