<?php

namespace App\Import;

use App\Snippet\Snippet;
use App\Snippet\SnippetInput;
use App\Snippet\SnippetRepository;
use Nativella\Nativella;

/**
 * Turns text files into snippets, named after the files: one picked in the
 * native file dialog, or several dropped on the window. A notification tells
 * what happened, even when the window is in the background.
 */
final readonly class SnippetImporter
{
    public function __construct(
        private Nativella $native,
        private SnippetRepository $repository,
    ) {
    }

    /**
     * Null when the user closed the dialog without picking a file.
     */
    public function importChosenFile(): ?ImportResult
    {
        $path = $this->native->dialog->openFile('Import a text file');

        return null === $path ? null : $this->importFiles([$path]);
    }

    /**
     * @param list<string> $paths absolute
     */
    public function importFiles(array $paths): ImportResult
    {
        $imported = [];
        $rejected = [];
        foreach ($paths as $path) {
            $outcome = $this->importFile($path);
            if ($outcome instanceof Snippet) {
                $imported[] = $outcome;
            } else {
                $rejected[] = $outcome;
            }
        }

        $result = new ImportResult($imported, $rejected);
        $this->native->notification->send($result->summary(), $result->details());

        return $result;
    }

    private function importFile(string $path): Snippet|RejectedFile
    {
        $name = basename($path);

        // A dropped directory, or a file deleted since.
        if (!is_file($path)) {
            return new RejectedFile($name, 'It is not a file.');
        }
        if (!is_readable($path)) {
            return new RejectedFile($name, 'It cannot be read.');
        }

        // One byte more than allowed is enough to reject a file that is too
        // large, without loading it whole.
        $content = file_get_contents($path, false, null, 0, SnippetInput::MAX_CONTENT_SIZE + 1);
        if (false === $content) {
            return new RejectedFile($name, 'It cannot be read.');
        }

        $input = new SnippetInput($name, $content);
        $errors = $input->validate();
        if ([] !== $errors) {
            return new RejectedFile($name, implode(' ', $errors));
        }

        return $this->repository->add($input->title, $input->content, new \DateTimeImmutable());
    }
}
