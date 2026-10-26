<?php

namespace Nativella;

use Nativella\Client\ExtensionClient;

/**
 * Every API of Nativella, for applications without a service container:
 *
 *     $native = Nativella::create();
 *     $native->notification->send('Saved!');
 *
 * With a container, inject the API classes (Window, Dialog…) instead.
 */
final readonly class Nativella
{
    public App $app;
    public BarcodeScanner $barcodeScanner;
    public Biometrics $biometrics;
    public ChildProcess $childProcess;
    public Clipboard $clipboard;
    public Dialog $dialog;
    public Geolocation $geolocation;
    public GlobalShortcut $globalShortcut;
    public Haptics $haptics;
    public Menu $menu;
    public Notification $notification;
    public Page $page;
    public Power $power;
    public Screen $screen;
    public Secrets $secrets;
    public Settings $settings;
    public Shell $shell;
    public System $system;
    public Timer $timer;
    public Tray $tray;
    public Window $window;

    public function __construct(Client $client)
    {
        $this->app = new App($client);
        $this->barcodeScanner = new BarcodeScanner($client);
        $this->biometrics = new Biometrics($client);
        $this->childProcess = new ChildProcess($client);
        $this->clipboard = new Clipboard($client);
        $this->dialog = new Dialog($client);
        $this->geolocation = new Geolocation($client);
        $this->globalShortcut = new GlobalShortcut($client);
        $this->haptics = new Haptics($client);
        $this->menu = new Menu($client);
        $this->notification = new Notification($client);
        $this->page = new Page($client);
        $this->power = new Power($client);
        $this->screen = new Screen($client);
        $this->secrets = new Secrets($client);
        $this->settings = new Settings($client);
        $this->shell = new Shell($client);
        $this->system = new System($client);
        $this->timer = new Timer($client);
        $this->tray = new Tray($client);
        $this->window = new Window($client);
    }

    public static function create(): self
    {
        return new self(new ExtensionClient());
    }
}
