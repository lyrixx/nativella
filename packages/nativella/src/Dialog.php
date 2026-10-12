<?php

namespace Nativella;

use Nativella\Dialog\FileFilter;
use Nativella\Dialog\MessageKind;

/**
 * Message boxes and file pickers. They are modal to the window that made the
 * request, and PHP waits for the answer.
 */
final readonly class Dialog
{
    public function __construct(
        private Client $client,
    ) {
    }

    public function message(string $message, ?string $title = null, MessageKind $kind = MessageKind::Info): void
    {
        $this->client->call('dialog.message', ['message' => $message, 'title' => $title, 'kind' => $kind->value]);
    }

    /**
     * Asks with OK and Cancel buttons, or the given labels.
     */
    public function confirm(string $message, ?string $title = null, MessageKind $kind = MessageKind::Info, ?string $okLabel = null, ?string $cancelLabel = null): bool
    {
        return $this->client->call('dialog.confirm', [
            'message' => $message,
            'title' => $title,
            'kind' => $kind->value,
            'okLabel' => $okLabel,
            'cancelLabel' => $cancelLabel,
        ])['confirmed'];
    }

    /**
     * Asks with Yes and No buttons.
     */
    public function ask(string $message, ?string $title = null, MessageKind $kind = MessageKind::Info): bool
    {
        return $this->client->call('dialog.ask', ['message' => $message, 'title' => $title, 'kind' => $kind->value])['answer'];
    }

    /**
     * @param array<FileFilter> $filters
     *
     * @return string|null the absolute path of the file, null if the user cancelled
     */
    public function openFile(?string $title = null, ?string $directory = null, array $filters = []): ?string
    {
        return $this->client->call('dialog.openFile', self::files($title, $directory, null, $filters))['path'];
    }

    /**
     * @param array<FileFilter> $filters
     *
     * @return list<string> the absolute paths of the files, none if the user cancelled
     */
    public function openFiles(?string $title = null, ?string $directory = null, array $filters = []): array
    {
        return $this->client->call('dialog.openFiles', self::files($title, $directory, null, $filters))['paths'];
    }

    /**
     * @return string|null the absolute path of the directory, null if the user cancelled
     */
    public function openDirectory(?string $title = null, ?string $directory = null): ?string
    {
        return $this->client->call('dialog.openDirectory', self::files($title, $directory, null, []))['path'];
    }

    /**
     * @param array<FileFilter> $filters
     *
     * @return string|null where the user wants the file, null if they cancelled
     */
    public function saveFile(?string $title = null, ?string $directory = null, ?string $fileName = null, array $filters = []): ?string
    {
        return $this->client->call('dialog.saveFile', self::files($title, $directory, $fileName, $filters))['path'];
    }

    /**
     * @param array<FileFilter> $filters
     *
     * @return array<string, mixed>
     */
    private static function files(?string $title, ?string $directory, ?string $fileName, array $filters): array
    {
        return [
            'title' => $title,
            'directory' => $directory,
            'fileName' => $fileName,
            // A list, or JSON would make it an object.
            'filters' => array_values(array_map(static fn (FileFilter $filter): array => $filter->toArray(), $filters)),
        ];
    }
}
