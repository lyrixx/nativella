<?php

namespace Nativella\Laravel;

use Illuminate\Contracts\Foundation\Application;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\Facades\Facade;
use Illuminate\Support\ServiceProvider;
use Illuminate\View\Compilers\BladeCompiler;
use Nativella\App;
use Nativella\BarcodeScanner;
use Nativella\Biometrics;
use Nativella\ChildProcess;
use Nativella\Client;
use Nativella\Client\ExtensionClient;
use Nativella\Clipboard;
use Nativella\Dialog;
use Nativella\Event\AppBooted;
use Nativella\Geolocation;
use Nativella\GlobalShortcut;
use Nativella\Haptics;
use Nativella\Laravel\Facades\Nativella as NativellaFacade;
use Nativella\Laravel\Listeners\MigrateOnBootListener;
use Nativella\Menu;
use Nativella\Nativella;
use Nativella\Nfc;
use Nativella\Notification;
use Nativella\Page;
use Nativella\Power;
use Nativella\Runtime;
use Nativella\Screen;
use Nativella\Secrets;
use Nativella\Settings;
use Nativella\Share;
use Nativella\Shell;
use Nativella\System;
use Nativella\Timer;
use Nativella\Tray;
use Nativella\Window;

/**
 * Runs the application in Nativella: the native APIs in the container, native
 * events dispatched as Laravel events, the database in the data directory of
 * the application, migrated at boot.
 */
final class NativellaServiceProvider extends ServiceProvider
{
    /**
     * The services built on the client.
     */
    private const array APIS = [
        App::class,
        BarcodeScanner::class,
        Biometrics::class,
        ChildProcess::class,
        Clipboard::class,
        Dialog::class,
        Geolocation::class,
        GlobalShortcut::class,
        Haptics::class,
        Menu::class,
        Nativella::class,
        Nfc::class,
        Notification::class,
        Page::class,
        Power::class,
        Screen::class,
        Secrets::class,
        Settings::class,
        Share::class,
        Shell::class,
        System::class,
        Timer::class,
        Tray::class,
        Window::class,
    ];

    private const string CONFIG = __DIR__.'/../config/nativella.php';

    public function register(): void
    {
        $this->mergeConfigFrom(self::CONFIG, 'nativella');

        // Before the binding: once bound, rebinding() would build the client.
        $this->app->rebinding(Client::class, static function (Application $app): void {
            // Another client (Nativella::fake()): the APIs, and the facades
            // that hold them, must use it.
            foreach (self::APIS as $api) {
                $app->forgetInstance($api);
                Facade::clearResolvedInstance($api);
            }
        });
        $this->app->singleton(Client::class, ExtensionClient::class);
        foreach (self::APIS as $api) {
            $this->app->singleton($api);
        }

        // Once every provider has registered, before any boots: the
        // configuration is final, no connection is open and nothing is
        // encrypted yet.
        $this->app->booting($this->storeDatabaseInDataDir(...));
        $this->app->booting($this->useInstallationSecret(...));
    }

    public function boot(): void
    {
        $this->publishes([self::CONFIG => $this->app->configPath('nativella.php')], 'nativella-config');

        $this->loadRoutesFrom(__DIR__.'/../routes/nativella.php');

        // Before the listeners of the application, which may use the
        // database: the providers of packages boot before the application's,
        // and discovered listeners come last.
        Event::listen(AppBooted::class, MigrateOnBootListener::class);

        $this->callAfterResolving('blade.compiler', static function (BladeCompiler $blade): void {
            $blade->if('native', NativellaFacade::isNative(...));
        });
    }

    /**
     * Encrypts and signs with the secret of the installation, whatever
     * APP_KEY and APP_PREVIOUS_KEYS say: the .env file ships with the
     * application, so its keys would be the same for every user, and anyone
     * holding the application could forge the cookies of any installation.
     */
    private function useInstallationSecret(Application $app): void
    {
        $secret = Runtime::secret();
        if (null === $secret) {
            return;
        }

        $config = $app->make('config');
        // The 32 bytes the 64 hexadecimal characters stand for, or the first
        // 16 of them for an AES-128 cipher.
        $bytes = pack('H*', $secret);
        if (str_starts_with(strtolower((string) $config->get('app.cipher')), 'aes-128')) {
            $bytes = substr($bytes, 0, 16);
        }

        $config->set([
            'app.key' => 'base64:'.base64_encode($bytes),
            'app.previous_keys' => [],
        ]);
    }

    /**
     * Points the "sqlite" connection at the data directory, and makes it the
     * default one: the user's data must survive an update of the application.
     */
    private function storeDatabaseInDataDir(Application $app): void
    {
        $dataDir = Runtime::dataDir();
        $config = $app->make('config');
        if (null === $dataDir || !$config->get('nativella.database')) {
            return;
        }

        $database = $dataDir.\DIRECTORY_SEPARATOR.'database.sqlite';
        if (!is_file($database)) {
            touch($database);
        }

        $config->set([
            'database.connections.sqlite' => [
                ...$config->get('database.connections.sqlite', []),
                'driver' => 'sqlite',
                // DB_URL, set for a web deployment, would win over the file.
                'url' => null,
                'database' => $database,
            ],
            'database.default' => 'sqlite',
            // config/queue.php reads DB_CONNECTION for these, not the default.
            'queue.batching.database' => 'sqlite',
            'queue.failed.database' => 'sqlite',
        ]);
    }
}
