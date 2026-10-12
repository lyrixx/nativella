<?php

namespace App\Http;

/**
 * Maps a method and a path to a handler. A path may hold an "{id}"
 * placeholder, which matches digits and reaches the handler in the request:
 *
 *     $router->post('/snippets/{id}/delete', static fn (Request $request): Response => … $request->id() …);
 */
final class Router
{
    /** @var array<string, array<string, \Closure(Request): Response>> pattern → method → handler */
    private array $routes = [];

    /**
     * @param \Closure(Request): Response $handler
     */
    public function get(string $path, \Closure $handler): void
    {
        $this->routes[self::pattern($path)]['GET'] = $handler;
    }

    /**
     * @param \Closure(Request): Response $handler
     */
    public function post(string $path, \Closure $handler): void
    {
        $this->routes[self::pattern($path)]['POST'] = $handler;
    }

    /**
     * Null when no route has this path.
     */
    public function dispatch(Request $request): ?Response
    {
        foreach ($this->routes as $pattern => $handlers) {
            if (1 !== preg_match($pattern, $request->path, $matches)) {
                continue;
            }

            $handler = $handlers[$request->method] ?? null;
            if (null === $handler) {
                return new Response('Method Not Allowed', 405, [
                    'Allow' => implode(', ', array_keys($handlers)),
                    'Content-Type' => 'text/plain; charset=UTF-8',
                ]);
            }

            if (isset($matches['id'])) {
                $request = $request->withId((int) $matches['id']);
            }

            return $handler($request);
        }

        return null;
    }

    private static function pattern(string $path): string
    {
        return '#^'.str_replace('\{id\}', '(?<id>\d+)', preg_quote($path, '#')).'$#';
    }
}
