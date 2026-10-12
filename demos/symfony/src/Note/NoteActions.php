<?php

namespace App\Note;

use App\Entity\Note;
use App\Repository\NoteRepository;
use Nativella\Dialog;
use Nativella\Dialog\FileFilter;
use Nativella\Dialog\MessageKind;
use Nativella\Notification;
use Nativella\Page;
use Symfony\Component\Filesystem\Filesystem;
use Symfony\Component\String\Slugger\SluggerInterface;

/**
 * What both the pages and the native menus do to notes, with native dialogs.
 * The caller flushes.
 */
final readonly class NoteActions
{
    public function __construct(
        private Dialog $dialog,
        private Filesystem $filesystem,
        private NoteImporter $importer,
        private NoteRepository $notes,
        private Notification $notification,
        private Page $page,
        private SluggerInterface $slugger,
    ) {
    }

    /**
     * @return bool false if the user changed their mind
     */
    public function delete(Note $note): bool
    {
        if (!$this->dialog->confirm(\sprintf('Delete "%s"? This cannot be undone.', $note->getTitle()), 'Delete the note', MessageKind::Warning, 'Delete')) {
            return false;
        }

        $this->notes->remove($note);

        return true;
    }

    /**
     * @return string|null where the note was saved, null if the user cancelled
     */
    public function export(Note $note): ?string
    {
        $path = $this->dialog->saveFile(
            'Export the note',
            fileName: $this->slugger->slug($note->getTitle())->lower().'.md',
            filters: [new FileFilter('Markdown', ['md'])],
        );

        if (null === $path) {
            return null;
        }

        $this->filesystem->dumpFile($path, $note->toMarkdown());
        $this->notification->send('Note exported', \sprintf('"%s" was saved to %s.', $note->getTitle(), $path));

        return $path;
    }

    public function importWithDialog(): ImportResult
    {
        return $this->importer->import($this->dialog->openFiles('Import notes', filters: [new FileFilter('Notes', NoteImporter::EXTENSIONS)]));
    }

    /**
     * Tells the open pages that the notes changed behind their back (from a
     * native menu, a dropped file…), so that they reload.
     *
     * @param array<string, mixed> $detail
     */
    public function notifyPages(array $detail = []): void
    {
        $this->page->dispatch('notes.changed', $detail);
    }
}
