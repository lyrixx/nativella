<?php

namespace App;

use App\Controller\SnippetController;
use App\Controller\SystemController;
use App\Http\CsrfToken;
use App\Http\Request;
use App\Http\Response;
use App\Http\Router;
use App\Import\SnippetImporter;
use App\Native\DeleteConfirmation;
use App\Snippet\SnippetRepository;
use App\View\Flash;
use App\View\View;
use Doctrine\DBAL\Connection;
use Nativella\Nativella;
use Nativella\Runtime;

/**
 * The pages of the application: a request of the webview, or of a browser,
 * gets its response.
 */
final readonly class Application
{
    public function __construct(
        private Connection $connection,
        private string $databasePath,
        private SnippetImporter $importer,
        private Nativella $native,
        private SnippetRepository $repository,
        private string $templateDir,
    ) {
    }

    public function handle(Request $request): Response
    {
        $csrfToken = new CsrfToken();
        $flash = new Flash();
        $view = new View($csrfToken, $request->path, $flash, $this->native, $this->templateDir);

        // Every form that posts changes something: in a browser, a page of
        // another site must not post it for the user.
        if ('POST' === $request->method && !$csrfToken->isValid($request)) {
            return new Response('The form expired: reload the page, and send it again.', 403, ['Content-Type' => 'text/plain; charset=UTF-8']);
        }

        $snippets = new SnippetController(new DeleteConfirmation($this->native), $flash, $this->importer, $this->native, $this->repository, $view);
        $system = new SystemController($this->connection, $this->databasePath, $this->native, $view);

        $router = new Router();
        $router->get('/', $snippets->index(...));
        $router->get('/snippets/new', $snippets->new(...));
        $router->post('/snippets', $snippets->create(...));
        $router->post('/snippets/{id}/delete', $snippets->delete(...));
        $router->get('/snippets/{id}/download', $snippets->download(...));
        $router->get('/system', $system->show(...));
        // The clipboard and the file dialog only exist in Nativella. On phones, the
        // file dialog picks a content URI, not a file PHP can read: no import there.
        if ($this->native->isNative()) {
            $router->post('/snippets/{id}/copy', $snippets->copy(...));
        }
        if ($this->native->isNative() && !Runtime::isMobile()) {
            $router->post('/snippets/import', $snippets->import(...));
        }

        return $router->dispatch($request)
            ?? $view->page('Not found', 'not-found', ['message' => 'This page does not exist.'], 404);
    }
}
