<?php

namespace App\Http\Controllers;

use App\Tasks\ExportFailed;
use App\Tasks\TaskExporter;
use Illuminate\Http\RedirectResponse;
use Psr\Log\LoggerInterface;

final readonly class ExportTasksController
{
    public function __invoke(TaskExporter $exporter, LoggerInterface $logger): RedirectResponse
    {
        try {
            $path = $exporter->export();
        } catch (ExportFailed $e) {
            $logger->warning('The tasks could not be exported.', ['exception' => $e]);

            return to_route('tasks.index')->with('error', $e->getMessage());
        }

        if (null === $path) {
            return to_route('tasks.index');
        }

        return to_route('tasks.index')->with('status', \sprintf('Tasks exported to %s.', $path));
    }
}
