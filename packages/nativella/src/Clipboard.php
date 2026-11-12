<?php

namespace Nativella;

use Nativella\Exception\UnexpectedResponseException;

final readonly class Clipboard
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Null when the clipboard holds no text, because it is empty or holds an
     * image.
     */
    public function readText(): ?string
    {
        return $this->client->call('clipboard.readText')['text'];
    }

    public function writeText(string $text): void
    {
        $this->client->call('clipboard.writeText', ['text' => $text]);
    }

    /**
     * @param string|null $altText what applications that do not read HTML paste
     */
    public function writeHtml(string $html, ?string $altText = null): void
    {
        $this->client->call('clipboard.writeHtml', ['html' => $html, 'altText' => $altText]);
    }

    /**
     * PNG bytes, null when the clipboard holds no image.
     */
    public function readImage(): ?string
    {
        $png = $this->client->call('clipboard.readImage')['png'];
        if (null === $png) {
            return null;
        }

        $bytes = base64_decode($png, true);
        if (false === $bytes) {
            throw new UnexpectedResponseException('The runtime sent a clipboard image that is not base64.');
        }

        return $bytes;
    }

    /**
     * @param string $png the bytes of a PNG image
     */
    public function writeImage(string $png): void
    {
        $this->client->call('clipboard.writeImage', ['png' => base64_encode($png)]);
    }

    public function clear(): void
    {
        $this->client->call('clipboard.clear');
    }
}
