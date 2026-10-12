<?php

namespace App\EventListener;

use App\Native\Desktop;
use App\Note\NoteActions;
use App\Repository\NoteRepository;
use Doctrine\ORM\EntityManagerInterface;
use Nativella\Event\MenuItemClicked;
use Nativella\Shell;
use Nativella\Window;
use Psr\Log\LoggerInterface;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

/**
 * The clicks on the items of the application menu, the tray menu and the
 * context menus of the notes ("note.<action>:<id>"). Links navigate by
 * themselves.
 */
#[AsEventListener]
final readonly class MenuListener
{
    public function __construct(
        private Desktop $desktop,
        private EntityManagerInterface $entityManager,
        private LoggerInterface $logger,
        private NoteActions $actions,
        private NoteRepository $notes,
        private Shell $shell,
        private Window $window,
    ) {
    }

    public function __invoke(MenuItemClicked $event): void
    {
        if (preg_match('/^note\.(?<action>\w+):(?<id>\d+)$/', $event->id, $matches)) {
            $this->onNote($matches['action'], (int) $matches['id']);

            return;
        }

        match ($event->id) {
            'file.import' => $this->import(),
            'help.website' => $this->shell->openUrl('https://github.com/lyrixx/nativella'),
            'tray.show' => $this->desktop->showMainWindow(),
            'view.alwaysOnTop' => $this->desktop->setAlwaysOnTop(true === $event->checked),
            'view.devtools' => $this->window->openDevtools(),
            'view.reload' => $this->window->reload(),
            // Links (file.new, tray.new…) and system roles.
            default => null,
        };
    }

    private function import(): void
    {
        $result = $this->actions->importWithDialog();
        if ([] === $result->notes) {
            return;
        }

        $this->entityManager->flush();
        $this->desktop->refreshTray();
        $this->actions->notifyPages(['created' => $result->notes[0]->getId()]);
    }

    private function onNote(string $action, int $id): void
    {
        // The note may be gone since its menu opened: deleted from another window.
        $note = $this->notes->find($id);
        if (null === $note) {
            $this->logger->warning('The note {id} of the context menu does not exist anymore.', ['id' => $id]);

            return;
        }

        switch ($action) {
            case 'delete':
                if (!$this->actions->delete($note)) {
                    return;
                }
                $this->entityManager->flush();
                $this->desktop->refreshTray();
                $this->actions->notifyPages(['deleted' => $id]);
                break;
            case 'duplicate':
                $copy = $note->duplicate();
                $this->notes->add($copy);
                $this->entityManager->flush();
                $this->desktop->refreshTray();
                $this->actions->notifyPages(['created' => $copy->getId()]);
                break;
            case 'export':
                $this->actions->export($note);
                break;
            case 'pin':
                if ($note->isPinned()) {
                    $note->unpin();
                } else {
                    $note->pin();
                }
                $this->entityManager->flush();
                $this->actions->notifyPages();
                break;
            default:
                $this->logger->warning('Unknown note action "{action}".', ['action' => $action]);
        }
    }
}
