<?php

/*
 * The front controller: every request of the webview (or of php -S) and
 * every native event go through it.
 */

use App\Controller\SnippetController;
use App\Controller\SystemController;
use App\Database;
use App\Http\Request;
use App\Http\Router;
use App\Import\SnippetImporter;
use App\Native\DeleteConfirmation;
use App\Native\NativeEventHandler;
use App\Snippet\SnippetRepository;
use App\Snippet\SnippetSchema;
use App\View\Flash;
use App\View\View;
use Nativella\Event\EventFactory;
use Nativella\Nativella;
use Nativella\Runtime;

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
// checking Runtime::isNative().
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

$request = Request::fromGlobals();
$flash = new Flash();
$view = new View($request->path, $flash, $projectDir.'/templates');

$snippets = new SnippetController(new DeleteConfirmation($native), $flash, $importer, $native, $repository, $view);
$system = new SystemController($connection, $databasePath, $native, $view);

$router = new Router();
$router->get('/', $snippets->index(...));
$router->get('/snippets/new', $snippets->new(...));
$router->post('/snippets', $snippets->create(...));
$router->post('/snippets/{id}/delete', $snippets->delete(...));
$router->get('/snippets/{id}/download', $snippets->download(...));
$router->get('/system', $system->show(...));
// The clipboard and the file dialog only exist in Nativella. On phones, the
// file dialog picks a content URI, not a file PHP can read: no import there.
if (Runtime::isNative()) {
    $router->post('/snippets/{id}/copy', $snippets->copy(...));
}
if (Runtime::isNative() && !Runtime::isMobile()) {
    $router->post('/snippets/import', $snippets->import(...));
}

$response = $router->dispatch($request)
    ?? $view->page('Not found', 'not-found', ['message' => 'This page does not exist.'], 404);
$response->send();
