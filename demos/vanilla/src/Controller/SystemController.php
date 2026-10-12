<?php

namespace App\Controller;

use App\Http\Response;
use App\View\View;
use Doctrine\DBAL\Connection;
use Nativella\Nativella;
use Nativella\Runtime;

/**
 * What Nativella knows about the application, the system and the displays.
 */
final readonly class SystemController
{
    public function __construct(
        private Connection $connection,
        private string $databasePath,
        private Nativella $native,
        private View $view,
    ) {
    }

    public function show(): Response
    {
        $isNative = Runtime::isNative();

        return $this->view->page('System', 'system', [
            'app' => $isNative ? $this->native->app->info() : null,
            'databasePath' => $this->databasePath,
            'dataDir' => Runtime::dataDir(),
            'runtimeVersion' => Runtime::version(),
            'screens' => $isNative ? $this->native->screen->all() : [],
            'sqliteVersion' => (string) $this->connection->fetchOne('SELECT sqlite_version()'),
            'system' => $isNative ? $this->native->system->info() : null,
            'window' => Runtime::window(),
        ]);
    }
}
