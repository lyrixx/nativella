<?php

namespace App\Listeners;

use App\Desktop\MainWindow;
use App\Desktop\MenuAction;
use App\Tasks\ExportFailed;
use App\Tasks\TaskExporter;
use Nativella\Dialog;
use Nativella\Dialog\MessageKind;
use Nativella\Event\MenuItemClicked;
use Nativella\Notification;
use Psr\Log\LoggerInterface;

final readonly class RunMenuAction
{
    public function __construct(
        private Dialog $dialog,
        private TaskExporter $exporter,
        private LoggerInterface $logger,
        private MainWindow $mainWindow,
        private Notification $notification,
    ) {
    }

    public function handle(MenuItemClicked $event): void
    {
        match (MenuAction::tryFrom($event->id)) {
            MenuAction::ExportTasks => $this->exportTasks(),
            MenuAction::ShowWindow => $this->mainWindow->show(),
            // A link or a system role: the runtime did the job.
            null => null,
        };
    }

    private function exportTasks(): void
    {
        try {
            $path = $this->exporter->export();
        } catch (ExportFailed $e) {
            $this->logger->warning('The tasks could not be exported.', ['exception' => $e]);
            $this->dialog->message($e->getMessage(), 'Export failed', MessageKind::Error);

            return;
        }

        if (null !== $path) {
            $this->notification->send('Tasks exported', $path);
        }
    }
}
