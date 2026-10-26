<?php

namespace App\Entity;

use App\Repository\NoteRepository;
use Doctrine\DBAL\Types\Types;
use Doctrine\ORM\Mapping as ORM;

#[ORM\Entity(repositoryClass: NoteRepository::class)]
class Note
{
    public const int TITLE_MAX_LENGTH = 200;

    #[ORM\Id]
    #[ORM\GeneratedValue]
    #[ORM\Column]
    private ?int $id = null;

    #[ORM\Column(length: self::TITLE_MAX_LENGTH)]
    private string $title;

    #[ORM\Column(type: Types::TEXT)]
    private string $body;

    #[ORM\Column]
    private bool $pinned = false;

    #[ORM\Column]
    private \DateTimeImmutable $createdAt;

    #[ORM\Column]
    private \DateTimeImmutable $updatedAt;

    public function __construct(string $title, string $body)
    {
        $this->title = $title;
        $this->body = $body;
        $this->createdAt = new \DateTimeImmutable();
        $this->updatedAt = $this->createdAt;
    }

    /**
     * A note from a Markdown or text file: its first "# Heading" is the title,
     * the file name otherwise.
     */
    public static function fromFile(string $fileName, string $content): self
    {
        $content = str_replace("\r\n", "\n", $content);

        if (preg_match('/\A\s*#\s+(?<title>[^\n]+)\n?(?<body>.*)\z/s', $content, $matches)) {
            return new self(self::truncate(trim($matches['title'])), trim($matches['body']));
        }

        $title = pathinfo($fileName, \PATHINFO_FILENAME);

        return new self(self::truncate('' === $title ? 'Untitled' : $title), trim($content));
    }

    public function duplicate(): self
    {
        return new self(self::truncate($this->title.' (copy)'), $this->body);
    }

    public function edit(string $title, string $body): void
    {
        $this->title = $title;
        $this->body = $body;
        $this->updatedAt = new \DateTimeImmutable();
    }

    /**
     * Ends the note with a link to the place, on OpenStreetMap.
     */
    public function addLocation(float $latitude, float $longitude): void
    {
        // Five decimals: about a meter.
        $coordinates = \sprintf('%.5F, %.5F', $latitude, $longitude);
        $map = \sprintf('https://www.openstreetmap.org/?mlat=%1$.5F&mlon=%2$.5F#map=17/%1$.5F/%2$.5F', $latitude, $longitude);
        $location = "📍 [{$coordinates}]({$map})";

        $body = rtrim($this->body);
        $this->edit($this->title, '' === $body ? $location : "{$body}\n\n{$location}");
    }

    /**
     * Ends the note with what a barcode holds: a link when it is a web
     * address, a code otherwise.
     */
    public function addCode(string $content): void
    {
        if (false !== filter_var($content, \FILTER_VALIDATE_URL) && 1 === preg_match('#^https?://#i', $content)) {
            $code = "📷 <{$content}>";
        } else {
            // Longer than any run of backticks in the content, or it would end the span.
            $fence = str_repeat('`', 1 + max(array_map(strlen(...), preg_match_all('/`+/', $content, $runs) ? $runs[0] : [''])));
            // A span strips one space on each side: a backtick at the end of the content needs one to stay.
            $space = str_starts_with($content, '`') || str_ends_with($content, '`') ? ' ' : '';
            $code = "📷 {$fence}{$space}{$content}{$space}{$fence}";
        }

        $body = rtrim($this->body);
        $this->edit($this->title, '' === $body ? $code : "{$body}\n\n{$code}");
    }

    public function pin(): void
    {
        $this->pinned = true;
    }

    public function unpin(): void
    {
        $this->pinned = false;
    }

    public function toMarkdown(): string
    {
        return "# {$this->title}\n\n{$this->body}\n";
    }

    public function getId(): ?int
    {
        return $this->id;
    }

    public function getTitle(): string
    {
        return $this->title;
    }

    public function getBody(): string
    {
        return $this->body;
    }

    public function isPinned(): bool
    {
        return $this->pinned;
    }

    public function getCreatedAt(): \DateTimeImmutable
    {
        return $this->createdAt;
    }

    public function getUpdatedAt(): \DateTimeImmutable
    {
        return $this->updatedAt;
    }

    private static function truncate(string $title): string
    {
        return mb_substr($title, 0, self::TITLE_MAX_LENGTH);
    }
}
