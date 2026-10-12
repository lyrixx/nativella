<?php

namespace App\Http;

final readonly class Request
{
    /**
     * @param array<mixed> $query $_GET
     * @param array<mixed> $form  $_POST
     */
    public function __construct(
        public string $method,
        /** Decoded, without the query string. */
        public string $path,
        private array $query = [],
        private array $form = [],
        private string $accept = '',
        /** The "{id}" of the route, which the router found in the path. */
        private ?int $id = null,
    ) {
    }

    public static function fromGlobals(): self
    {
        $path = parse_url($_SERVER['REQUEST_URI'] ?? '/', \PHP_URL_PATH);

        return new self(
            $_SERVER['REQUEST_METHOD'] ?? 'GET',
            \is_string($path) ? rawurldecode($path) : '/',
            $_GET,
            $_POST,
            $_SERVER['HTTP_ACCEPT'] ?? '',
        );
    }

    public function withId(int $id): self
    {
        return new self($this->method, $this->path, $this->query, $this->form, $this->accept, $id);
    }

    /**
     * The "{id}" of the route.
     */
    public function id(): int
    {
        return $this->id ?? throw new \LogicException(\sprintf('The route of "%s" has no "{id}" placeholder.', $this->path));
    }

    /**
     * A query string parameter, empty when it is missing or not a string (?q[]=…).
     */
    public function query(string $name): string
    {
        return self::string($this->query, $name);
    }

    /**
     * A form field, empty when it is missing or not a string.
     */
    public function form(string $name): string
    {
        return self::string($this->form, $name);
    }

    /**
     * Whether the page's JavaScript asked for JSON rather than a page.
     */
    public function acceptsJson(): bool
    {
        return str_contains($this->accept, 'application/json');
    }

    /**
     * @param array<mixed> $parameters
     */
    private static function string(array $parameters, string $name): string
    {
        $value = $parameters[$name] ?? '';

        return \is_string($value) ? $value : '';
    }
}
