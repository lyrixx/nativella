<?php

/*
 * The front controller: every request of the webview (or of php -S) and
 * every native event go through it.
 */

use App\Application;
use App\Database;
use App\Http\Request;
use App\Import\SnippetImporter;
use App\Native\NativeEventHandler;
use App\Snippet\SnippetRepository;
use App\Snippet\SnippetSchema;
use Nativella\Event\EventFactory;
use Nativella\Nativella;

require dirname(__DIR__).'/vendor/autoload.php';

// A blank window is all a desktop user would see of an uncaught exception.
set_exception_handler(static function (Throwable $exception): void {
    error_log((string) $exception);
    if (!headers_sent()) {
        http_response_code(500);
        header('Content-Type: text/html; charset=UTF-8');
    }

    echo <<<'HTML'
        <!DOCTYPE html>
        <html lang="en">
        <meta charset="UTF-8">
        <title>Error</title>
        <link rel="stylesheet" href="/app.css">
        <main class="empty">
            <h1>Something went wrong</h1>
            <p>The details are in the log of the application.</p>
            <p><a class="button" href="/">Back to the snippets</a></p>
        </main>
        HTML;
});

$projectDir = dirname(__DIR__);

$databasePath = Database::path($projectDir);
$connection = Database::connect($databasePath);
// Calls nothing until used: outside Nativella, the pages only use it after
// checking $native->isNative().
$native = Nativella::create();
$repository = new SnippetRepository($connection);
$schema = new SnippetSchema($connection);
$importer = new SnippetImporter($native, $repository);

// A native event: the runtime posts it to /_nativella/event, and only the
// runtime can (EventFactory checks $_SERVER['NATIVELLA_INTERNAL']).
if (null !== $event = EventFactory::fromGlobals()) {
    new NativeEventHandler($importer, $native, $schema)->handle($event);
    http_response_code(204);

    return;
}

// Nativella migrates on "app.booted", php -S has no such event.
$schema->migrateIfMissing();

new Application($connection, $databasePath, $importer, $native, $repository, $projectDir.'/templates')
    ->handle(Request::fromGlobals())
    ->send();
