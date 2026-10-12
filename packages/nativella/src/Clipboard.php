<?php

namespace Nativella;

/**
 * The system clipboard: text, HTML or an image.
 */
final readonly class Clipboard
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Null when the clipboard holds no text: it is empty, or holds an image.
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
     * The image of the clipboard as PNG bytes, null when it holds none.
     */
    public function readImage(): ?string
    {
        $png = $this->client->call('clipboard.readImage')['png'];
        if (null === $png) {
            return null;
        }

        // The runtime encodes valid base64.
        return base64_decode($png, true) ?: throw new \LogicException('The runtime sent a clipboard image that is not base64.');
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
