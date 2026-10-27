<?php

namespace App\Http;

final readonly class Response
{
    /**
     * @param array<string, string> $headers
     */
    public function __construct(
        public string $body = '',
        public int $status = 200,
        public array $headers = [],
    ) {
    }

    public static function html(string $html, int $status = 200): self
    {
        return new self($html, $status, ['Content-Type' => 'text/html; charset=UTF-8']);
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function json(array $data, int $status = 200): self
    {
        return new self(json_encode($data, \JSON_THROW_ON_ERROR | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE), $status, ['Content-Type' => 'application/json']);
    }

    /**
     * A file to save rather than show: a browser, or Nativella, asks where.
     */
    public static function download(string $content, string $fileName, string $contentType): self
    {
        // "filename" for the old clients, in ASCII; "filename*" in UTF-8 for the others.
        $ascii = preg_replace('/[^\x20-\x7E]|["\\\\]/u', '_', $fileName);

        return new self($content, 200, [
            'Content-Disposition' => \sprintf('attachment; filename="%s"; filename*=UTF-8\'\'%s', $ascii, rawurlencode($fileName)),
            'Content-Type' => $contentType,
        ]);
    }

    /**
     * After a form: "See Other" turns the next request into a GET, so that
     * reloading the page does not post the form again.
     */
    public static function redirect(string $url): self
    {
        return new self('', 303, ['Location' => $url]);
    }

    public function send(): void
    {
        http_response_code($this->status);
        foreach ($this->headers as $name => $value) {
            header($name.': '.$value);
        }

        echo $this->body;
    }
}
