<?php

namespace App\Note;

use App\Entity\Note;
use App\Repository\NoteRepository;
use Psr\Log\LoggerInterface;

/**
 * Creates notes from Markdown and text files: picked in a dialog, or dropped
 * on the window.
 */
final readonly class NoteImporter
{
    public const array EXTENSIONS = ['markdown', 'md', 'txt'];

    private const int MAX_SIZE = 1024 * 1024;

    public function __construct(
        private NoteRepository $notes,
        private LoggerInterface $logger,
    ) {
    }

    /**
     * @param list<string> $paths absolute
     */
    public function import(array $paths): ImportResult
    {
        $imported = [];
        $skipped = [];

        foreach ($paths as $path) {
            $file = new \SplFileInfo($path);

            if (!\in_array(strtolower($file->getExtension()), self::EXTENSIONS, true) || !$file->isFile() || $file->getSize() > self::MAX_SIZE) {
                $skipped[] = $file->getFilename();

                continue;
            }

            $content = @file_get_contents($path);
            if (false === $content) {
                $this->logger->warning('Could not read "{path}" to import it.', ['path' => $path, 'error' => error_get_last()['message'] ?? null]);
                $skipped[] = $file->getFilename();

                continue;
            }

            $note = Note::fromFile($file->getFilename(), $content);
            $this->notes->add($note);
            $imported[] = $note;
        }

        return new ImportResult($imported, $skipped);
    }
}
