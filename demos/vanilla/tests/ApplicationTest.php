<?php

namespace App\Tests;

use App\Application;
use App\Http\CsrfToken;
use App\Http\Request;
use App\Http\Response;
use App\Import\SnippetImporter;
use App\Snippet\SnippetRepository;
use App\Snippet\SnippetSchema;
use Doctrine\DBAL\DriverManager;
use Nativella\Client\FakeClient;
use Nativella\Nativella;
use PHPUnit\Framework\TestCase;

/**
 * The pages, in Nativella (with the fake client) and in a browser (with the
 * client of the runtime, which has no runtime to call).
 */
final class ApplicationTest extends TestCase
{
    private FakeClient $client;
    private SnippetRepository $repository;

    protected function setUp(): void
    {
        $this->client = new FakeClient();
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);
        session_unset();
    }

    public function testAFormPostedWithoutItsTokenChangesNothing(): void
    {
        $application = $this->application(new Nativella($this->client));

        $response = $application->handle(new Request('POST', '/snippets', form: ['title' => 'SSH', 'content' => 'ssh example.com']));

        $this->assertSame(403, $response->status);
        $this->assertSame([], $this->repository->search('', 10));
    }

    public function testASnippetIsSavedThenListed(): void
    {
        $application = $this->application(new Nativella($this->client));

        $response = $this->post($application, '/snippets', ['title' => 'SSH', 'content' => 'ssh example.com']);
        $this->assertSame(303, $response->status);
        $this->assertSame('/', $response->headers['Location']);

        $list = $application->handle(new Request('GET', '/'));
        $this->assertSame(200, $list->status);
        $this->assertStringContainsString('<h2>SSH</h2>', $list->body);
        $this->assertStringContainsString('Saved “SSH”.', $list->body);
    }

    public function testASnippetWithoutContentIsNotSaved(): void
    {
        $response = $this->post($this->application(new Nativella($this->client)), '/snippets', ['title' => 'SSH', 'content' => '']);

        $this->assertSame(422, $response->status);
        $this->assertStringContainsString('The content cannot be empty.', $response->body);
    }

    public function testInNativellaASnippetIsCopied(): void
    {
        $application = $this->application(new Nativella($this->client));
        $snippet = $this->repository->add('SSH', 'ssh example.com', new \DateTimeImmutable());

        $response = $application->handle(new Request('POST', "/snippets/{$snippet->id}/copy", form: [CsrfToken::FIELD => new CsrfToken()->value()], accept: 'application/json'));

        $this->assertSame('{"message":"Copied “SSH” to the clipboard."}', $response->body);
        $this->assertSame(['text' => 'ssh example.com'], $this->client->calls('clipboard.writeText')[0]->payload);
    }

    public function testInNativellaDeletingAsksFirst(): void
    {
        $application = $this->application(new Nativella($this->client));
        $snippet = $this->repository->add('SSH', 'ssh example.com', new \DateTimeImmutable());

        $this->post($application, "/snippets/{$snippet->id}/delete");
        $this->assertNotNull($this->repository->find($snippet->id), 'The user said no.');

        $this->client->respond('dialog.confirm', ['confirmed' => true]);
        $response = $this->post($application, "/snippets/{$snippet->id}/delete", ['q' => 'ssh']);

        $this->assertSame('/?q=ssh', $response->headers['Location']);
        $this->assertNull($this->repository->find($snippet->id));
    }

    public function testInABrowserTheNativeRoutesDoNotExist(): void
    {
        $application = $this->application(Nativella::create());
        $snippet = $this->repository->add('SSH', 'ssh example.com', new \DateTimeImmutable());

        $this->assertSame(404, $this->post($application, "/snippets/{$snippet->id}/copy")->status);
        $this->assertSame(404, $this->post($application, '/snippets/import')->status);
    }

    public function testInABrowserThePageAsksBeforeDeleting(): void
    {
        $application = $this->application(Nativella::create());
        $snippet = $this->repository->add('SSH', 'ssh example.com', new \DateTimeImmutable());

        $list = $application->handle(new Request('GET', '/'));
        $this->assertStringContainsString('data-confirm="Delete “SSH”? This cannot be undone."', $list->body);
        $this->assertStringContainsString('Running in a browser', $list->body);
        $this->assertStringNotContainsString('/copy', $list->body);

        $this->assertSame(303, $this->post($application, "/snippets/{$snippet->id}/delete")->status);
        $this->assertNull($this->repository->find($snippet->id));
    }

    public function testInABrowserTheSystemPageKnowsLittle(): void
    {
        $response = $this->application(Nativella::create())->handle(new Request('GET', '/system'));

        $this->assertSame(200, $response->status);
        $this->assertStringContainsString('only known when the app runs in Nativella', $response->body);
    }

    public function testInNativellaTheSystemPageShowsTheApplication(): void
    {
        $response = $this->application(new Nativella($this->client))->handle(new Request('GET', '/system'));

        $this->assertStringContainsString('<code>dev.nativella.app</code>', $response->body);
    }

    public function testAnUnknownPathIsNotFound(): void
    {
        $response = $this->application(new Nativella($this->client))->handle(new Request('GET', '/nowhere'));

        $this->assertSame(404, $response->status);
    }

    private function application(Nativella $native): Application
    {
        $connection = DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true]);
        new SnippetSchema($connection)->migrate();
        $this->repository = new SnippetRepository($connection);

        return new Application($connection, ':memory:', new SnippetImporter($native, $this->repository), $native, $this->repository, \dirname(__DIR__).'/templates');
    }

    /**
     * Posts the way a form of the pages does: with its CSRF token.
     *
     * @param array<string, string> $form
     */
    private function post(Application $application, string $path, array $form = []): Response
    {
        return $application->handle(new Request('POST', $path, form: [CsrfToken::FIELD => new CsrfToken()->value()] + $form));
    }
}
