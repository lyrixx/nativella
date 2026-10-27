<?php

namespace App\Controller;

use App\Http\Request;
use App\Http\Response;
use App\Import\SnippetImporter;
use App\Native\DeleteConfirmation;
use App\Snippet\SnippetInput;
use App\Snippet\SnippetRepository;
use App\View\Flash;
use App\View\View;
use Nativella\Nativella;
use Nativella\Runtime;

final readonly class SnippetController
{
    /** The list shows this many snippets at most: a search finds the others. */
    public const int LIST_LIMIT = 100;

    /** Asked by a native dialog in Nativella, by the browser outside. */
    public const string DELETE_QUESTION = 'Delete “%s”? This cannot be undone.';

    public function __construct(
        private DeleteConfirmation $deleteConfirmation,
        private Flash $flash,
        private SnippetImporter $importer,
        private Nativella $native,
        private SnippetRepository $repository,
        private View $view,
    ) {
    }

    public function index(Request $request): Response
    {
        $query = trim($request->query('q'));
        // One more than shown tells whether there are more.
        $snippets = $this->repository->search($query, self::LIST_LIMIT + 1);

        return $this->view->page('Snippets', 'snippets/index', [
            'hasMore' => \count($snippets) > self::LIST_LIMIT,
            'query' => $query,
            'snippets' => \array_slice($snippets, 0, self::LIST_LIMIT),
        ]);
    }

    public function new(): Response
    {
        return $this->view->page('New snippet', 'snippets/new', [
            'errors' => [],
            'input' => new SnippetInput('', ''),
        ]);
    }

    public function create(Request $request): Response
    {
        $input = SnippetInput::fromForm($request->form('title'), $request->form('content'));
        $errors = $input->validate();
        if ([] !== $errors) {
            return $this->view->page('New snippet', 'snippets/new', ['errors' => $errors, 'input' => $input], 422);
        }

        $snippet = $this->repository->add($input->title, $input->content, new \DateTimeImmutable());
        $this->flash->add(\sprintf('Saved “%s”.', $snippet->title));

        return Response::redirect('/');
    }

    /**
     * Only routed in Nativella. The page's JavaScript posts the form itself
     * and shows the message, without leaving the page.
     */
    public function copy(Request $request): Response
    {
        $snippet = $this->repository->find($request->id());
        if (null === $snippet) {
            return $this->notFound();
        }

        $this->native->clipboard->writeText($snippet->content);
        $message = \sprintf('Copied “%s” to the clipboard.', $snippet->title);

        if ($request->acceptsJson()) {
            return Response::json(['message' => $message]);
        }

        $this->flash->add($message);

        return Response::redirect('/');
    }

    public function download(Request $request): Response
    {
        $snippet = $this->repository->find($request->id());
        if (null === $snippet) {
            return $this->notFound();
        }

        $fileName = str_replace(['/', '\\'], '-', $snippet->title).'.txt';

        return Response::download($snippet->content, $fileName, 'text/plain; charset=UTF-8');
    }

    public function delete(Request $request): Response
    {
        $snippet = $this->repository->find($request->id());
        if (null === $snippet) {
            return $this->notFound();
        }

        $list = self::listUrl($request->form('q'));
        $question = \sprintf(self::DELETE_QUESTION, $snippet->title);
        if (Runtime::isNative() && !$this->deleteConfirmation->confirm($question)) {
            return Response::redirect($list);
        }

        $this->repository->delete($snippet->id);
        $this->flash->add(\sprintf('Deleted “%s”.', $snippet->title));

        return Response::redirect($list);
    }

    /**
     * Only routed in Nativella, on the desktop.
     */
    public function import(): Response
    {
        $result = $this->importer->importChosenFile();
        if (null !== $result) {
            $this->flash->add($result->message());
        }

        return Response::redirect('/');
    }

    private function notFound(): Response
    {
        return $this->view->page('Not found', 'not-found', ['message' => 'This snippet does not exist, or not anymore.'], 404);
    }

    /**
     * The list, filtered by the search it showed.
     */
    private static function listUrl(string $query): string
    {
        $query = trim($query);

        return '' === $query ? '/' : '/?'.http_build_query(['q' => $query]);
    }
}
