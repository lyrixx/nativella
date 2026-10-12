<?php

namespace Nativella;

use Nativella\Window\WindowInfo;
use Nativella\Window\WindowOptions;

/**
 * Opens and drives windows, each one known by its label.
 *
 * Without a label, a method acts on the window that made the request, or on
 * the focused one during a native event.
 */
final readonly class Window
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Opens a window, or focuses it if it is already open.
     *
     * The main window starts from the options of the "window" key of
     * extra.nativella in composer.json, which $options complete.
     */
    public function open(string $label, ?WindowOptions $options = null): WindowInfo
    {
        return WindowInfo::fromArray($this->client->call('window.open', ['label' => $label] + ($options?->toArray() ?? []))['window']);
    }

    public function close(?string $window = null): void
    {
        $this->client->call('window.close', ['window' => $window]);
    }

    /**
     * @return list<WindowInfo> sorted by label
     */
    public function all(): array
    {
        // @phpstan-ignore return.type (a JSON array, so a list: crates/nativella/src/native/window.rs)
        return array_map(WindowInfo::fromArray(...), $this->client->call('window.all')['windows']);
    }

    public function get(?string $window = null): WindowInfo
    {
        return WindowInfo::fromArray($this->client->call('window.get', ['window' => $window])['window']);
    }

    public function setTitle(string $title, ?string $window = null): void
    {
        $this->client->call('window.setTitle', ['title' => $title, 'window' => $window]);
    }

    public function setSize(float $width, float $height, ?string $window = null): void
    {
        $this->client->call('window.setSize', ['width' => $width, 'height' => $height, 'window' => $window]);
    }

    public function setPosition(float $x, float $y, ?string $window = null): void
    {
        $this->client->call('window.setPosition', ['x' => $x, 'y' => $y, 'window' => $window]);
    }

    public function setAlwaysOnTop(bool $enabled, ?string $window = null): void
    {
        $this->client->call('window.setAlwaysOnTop', ['enabled' => $enabled, 'window' => $window]);
    }

    public function setFullscreen(bool $enabled, ?string $window = null): void
    {
        $this->client->call('window.setFullscreen', ['enabled' => $enabled, 'window' => $window]);
    }

    public function setResizable(bool $enabled, ?string $window = null): void
    {
        $this->client->call('window.setResizable', ['enabled' => $enabled, 'window' => $window]);
    }

    /**
     * Shows a progress bar on the icon of the window, in the taskbar or the
     * dock, where the system has one (Windows, macOS, GNOME with libunity).
     *
     * @param int<0, 100>|null $percent null removes the bar
     */
    public function setProgress(?int $percent, ?string $window = null): void
    {
        $this->client->call('window.setProgress', ['percent' => $percent, 'window' => $window]);
    }

    public function center(?string $window = null): void
    {
        $this->client->call('window.center', ['window' => $window]);
    }

    public function minimize(?string $window = null): void
    {
        $this->client->call('window.minimize', ['window' => $window]);
    }

    public function maximize(?string $window = null): void
    {
        $this->client->call('window.maximize', ['window' => $window]);
    }

    public function unmaximize(?string $window = null): void
    {
        $this->client->call('window.unmaximize', ['window' => $window]);
    }

    public function show(?string $window = null): void
    {
        $this->client->call('window.show', ['window' => $window]);
    }

    public function hide(?string $window = null): void
    {
        $this->client->call('window.hide', ['window' => $window]);
    }

    public function focus(?string $window = null): void
    {
        $this->client->call('window.focus', ['window' => $window]);
    }

    public function reload(?string $window = null): void
    {
        $this->client->call('window.reload', ['window' => $window]);
    }

    /**
     * Opens the print dialog of the system for the page of the window.
     */
    public function print(?string $window = null): void
    {
        $this->client->call('window.print', ['window' => $window]);
    }

    /**
     * Writes the page of the window to a PDF file, without a dialog, as it
     * would print: @media print styles apply. Linux only, for now.
     *
     * @param string $path absolute
     */
    public function printToPdf(string $path, ?string $window = null): void
    {
        $this->client->call('window.printToPdf', ['window' => $window, 'path' => $path]);
    }

    /**
     * @param string $url a path of the application ("/settings"), or an absolute URL
     */
    public function navigate(string $url, ?string $window = null): void
    {
        $this->client->call('window.navigate', ['url' => $url, 'window' => $window]);
    }

    public function openDevtools(?string $window = null): void
    {
        $this->client->call('window.openDevtools', ['window' => $window]);
    }
}
