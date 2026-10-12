<?php

namespace App\Tests\Http;

use App\Http\Request;
use App\Http\Response;
use App\Http\Router;
use PHPUnit\Framework\TestCase;

final class RouterTest extends TestCase
{
    private Router $router;

    protected function setUp(): void
    {
        $this->router = new Router();
        $this->router->get('/', static fn (): Response => new Response('home'));
        $this->router->get('/snippets/new', static fn (): Response => new Response('form'));
        $this->router->post('/snippets/{id}/delete', static fn (Request $request): Response => new Response('deleted '.$request->id()));
    }

    public function testDispatchesByMethodAndPath(): void
    {
        $this->assertSame('home', $this->router->dispatch(new Request('GET', '/'))?->body);
        $this->assertSame('form', $this->router->dispatch(new Request('GET', '/snippets/new'))?->body);
    }

    public function testTheIdPlaceholderReachesTheHandlerInTheRequest(): void
    {
        $this->assertSame('deleted 42', $this->router->dispatch(new Request('POST', '/snippets/42/delete'))?->body);
    }

    public function testTheIdPlaceholderOnlyMatchesDigits(): void
    {
        $this->assertNull($this->router->dispatch(new Request('POST', '/snippets/abc/delete')));
        $this->assertNull($this->router->dispatch(new Request('POST', '/snippets/-1/delete')));
    }

    public function testARouteWithoutIdHasNone(): void
    {
        $this->router->get('/broken', static fn (Request $request): Response => new Response((string) $request->id()));

        $this->expectException(\LogicException::class);
        $this->expectExceptionMessage('The route of "/broken" has no "{id}" placeholder.');

        $this->router->dispatch(new Request('GET', '/broken'));
    }

    public function testNoRouteIsNull(): void
    {
        $this->assertNull($this->router->dispatch(new Request('GET', '/nope')));
        $this->assertNull($this->router->dispatch(new Request('GET', '/snippets/new/')));
    }

    public function testAnotherMethodIsNotAllowed(): void
    {
        $response = $this->router->dispatch(new Request('POST', '/snippets/new'));

        $this->assertSame(405, $response?->status);
        $this->assertSame('GET', $response->headers['Allow']);
    }
}
