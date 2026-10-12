<?php

namespace App\EventListener;

use App\Native\Desktop;
use App\Note\NoteActions;
use App\Note\NoteImporter;
use Doctrine\ORM\EntityManagerInterface;
use Nativella\Event\WindowFilesDropped;
use Nativella\Notification;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * Markdown and text files dropped on a window become notes.
 */
#[AsEventListener]
final readonly class DroppedFilesListener
{
    public function __construct(
        private Desktop $desktop,
        private EntityManagerInterface $entityManager,
        private NoteActions $actions,
        private NoteImporter $importer,
        private Notification $notification,
    ) {
    }

    public function __invoke(WindowFilesDropped $event): void
    {
        $result = $this->importer->import($event->paths);
        $this->notification->send('Nativella Notes', $result->summary());

        if ([] === $result->notes) {
            return;
        }

        $this->entityManager->flush();
        $this->desktop->refreshTray();
        $this->actions->notifyPages(['created' => $result->notes[0]->getId()]);
    }
}
