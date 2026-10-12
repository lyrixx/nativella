<?php

namespace App\Snippet;

/**
 * A snippet before it is saved: what the user typed in the form, or what an
 * imported file holds.
 */
final readonly class SnippetInput
{
    /** In characters. */
    public const int MAX_TITLE_LENGTH = 255;

    /** In bytes: 1 MiB. */
    public const int MAX_CONTENT_SIZE = 1024 * 1024;

    public function __construct(
        public string $title,
        public string $content,
    ) {
    }

    public static function fromForm(string $title, string $content): self
    {
        // Browsers send the line breaks of a textarea as CRLF.
        return new self(trim($title), str_replace("\r\n", "\n", $content));
    }

    /**
     * @return array<'content'|'title', string> the error of each invalid field, none when the input is valid
     */
    public function validate(): array
    {
        $errors = [];

        if (!self::isText($this->title)) {
            $errors['title'] = 'The title must be UTF-8 text.';
        } elseif ('' === trim($this->title)) {
            $errors['title'] = 'The title cannot be empty.';
        } elseif (mb_strlen($this->title) > self::MAX_TITLE_LENGTH) {
            $errors['title'] = \sprintf('The title cannot be longer than %d characters.', self::MAX_TITLE_LENGTH);
        }

        if (\strlen($this->content) > self::MAX_CONTENT_SIZE) {
            $errors['content'] = \sprintf('The content cannot be larger than %d MiB.', self::MAX_CONTENT_SIZE / 1024 / 1024);
        } elseif (!self::isText($this->content)) {
            $errors['content'] = 'The content must be UTF-8 text.';
        } elseif ('' === trim($this->content)) {
            $errors['content'] = 'The content cannot be empty.';
        }

        return $errors;
    }

    /**
     * Binary data (an image dropped on the window…) is rarely valid UTF-8, and
     * when it is, it holds NUL bytes.
     */
    private static function isText(string $value): bool
    {
        return mb_check_encoding($value, 'UTF-8') && !str_contains($value, "\0");
    }
}
