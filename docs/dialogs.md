# Dialogs

`Nativella\Dialog` shows the dialogs of the system: message boxes, questions and file pickers. They are modal to the window, and PHP waits for the answer: the method returns what the user chose.

```php
use Nativella\Dialog\FileFilter;
use Nativella\Dialog\MessageKind;

if ($dialog->confirm('Delete "Groceries"? This cannot be undone.', 'Delete note', MessageKind::Warning, okLabel: 'Delete')) {
    $notes->delete($note);
}

$path = $dialog->saveFile(
    title: 'Export note',
    fileName: 'groceries.md',
    filters: [new FileFilter('Markdown', ['md'])],
);
if (null !== $path) {
    file_put_contents($path, $note->markdown());
}
```

> [!WARNING]
> While a dialog is open, PHP waits, and so does every other request: one PHP thread serves them all. The files of `public/` keep loading, as the runtime serves them without PHP. Dialogs are for answers the user must give before anything else happens.

## Messages

| Method | Returns |
|---|---|
| `message(string $message, ?string $title = null, MessageKind $kind = MessageKind::Info): void` | When the user dismisses the dialog. |
| `confirm(string $message, ?string $title = null, MessageKind $kind = MessageKind::Info, ?string $okLabel = null, ?string $cancelLabel = null): bool` | `true` for OK. The buttons say OK and Cancel, unless you name them. |
| `ask(string $message, ?string $title = null, MessageKind $kind = MessageKind::Info): bool` | `true` for Yes, with Yes and No buttons. |

`Nativella\Dialog\MessageKind` sets the icon: `Info`, `Warning` or `Error`.

## Files

| Method | Returns |
|---|---|
| `openFile(?string $title = null, ?string $directory = null, list<FileFilter> $filters = []): ?string` | The absolute path of the chosen file. |
| `openFiles(?string $title = null, ?string $directory = null, list<FileFilter> $filters = []): list<string>` | The absolute paths of the chosen files. |
| `openDirectory(?string $title = null, ?string $directory = null): ?string` | The absolute path of the chosen directory. |
| `saveFile(?string $title = null, ?string $directory = null, ?string $fileName = null, list<FileFilter> $filters = []): ?string` | Where the user wants the file. The file is not created: writing it is up to you. |

A cancelled dialog returns null, or an empty list. `$directory` is where the dialog starts, `$fileName` the name `saveFile()` suggests.

`Nativella\Dialog\FileFilter` restricts the choice to some extensions, without the dot:

```php
use Nativella\Dialog\FileFilter;

$paths = $dialog->openFiles('Import pictures', filters: [
    new FileFilter('Images', ['png', 'jpg', 'jpeg', 'webp']),
]);
```

The paths are local files, readable by PHP right away: there is no upload. That is the main difference with an `<input type="file">`, which hands PHP a copy of the file in `$_FILES` rather than its path. Uploads go through PHP like any other request body, but they are untested so far.

## Dialogs from a native event

During a [native event](lifecycle.md#native-events), such as a menu click, the dialog is modal to the focused window. A common pattern opens a file from the menu, then shows it:

```php
use Nativella\Dialog\FileFilter;
use Nativella\Event\MenuItemClicked;

if ($event instanceof MenuItemClicked && 'open' === $event->id) {
    $path = $dialog->openFile('Open a note', filters: [new FileFilter('Markdown', ['md'])]);
    if (null !== $path) {
        $id = $notes->import($path);
        $window->navigate('/notes/'.$id);
    }
}
```
