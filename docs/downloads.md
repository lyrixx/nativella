# Downloads

A link or a form that answers a file to save works as in a browser: Nativella asks the user where to save it, and the page stays as it is. Nothing to call: the export of a web application keeps working.

```php
header('Content-Type: text/csv');
header('Content-Disposition: attachment; filename="notes.csv"');

echo $csv;
```

Symfony's `$this->file()`, `BinaryFileResponse` or `HeaderUtils::makeDisposition()`, and Laravel's `response()->download()` and `response()->streamDownload()`, answer the same header.

## What downloads

- **An attachment**: a response with `Content-Disposition: attachment` to a link or a form.
- **A link with a `download` attribute**, to a page or a file of the application: `<a href="/notes/export" download>`. Its response downloads, attachment or not.
- **A file the page makes itself**: a `blob:` or a `data:` URL in a link with a `download` attribute, as the JavaScript libraries that export a table or a chart do.

A `fetch()` or an XHR gets the response as it is, attachment or not, as in a browser: the script decides what to do with it. A response that is not a success, a `404` or a `500`, is shown as any other, not saved.

## The dialog

The save dialog opens in the downloads directory of the user, with the name of the file:

1. the `download` attribute of the link, when it has a value;
2. the `filename*` of `Content-Disposition`, in UTF-8 (`filename*=UTF-8''r%C3%A9sum%C3%A9.pdf`), then its `filename`;
3. the last segment of the URL: `report.pdf` for `/files/report.pdf`.

Only the name counts: a path in it (`../../.bashrc`) is cut to what follows the last `/` or `\`. The user can rename the file, and pick another directory.

Once the file is saved, PHP and the pages get `window.downloaded`, with the label of the `window` and the absolute `path` of the file: a `Nativella\Event\WindowDownloaded`, and a `nativella:window.downloaded` DOM event. Nothing happens when the user cancels.

```php
use Nativella\Event\WindowDownloaded;

#[AsEventListener]
public function __invoke(WindowDownloaded $event): void
{
    $this->notification->send('Export saved', basename($event->path));
}
```

To write a file PHP already holds without a link, call [`Dialog::saveFile()`](dialogs.md) and write it yourself.

## Platforms

Tested on Linux. macOS and Windows go through the same code, untested. Phones do not save downloads yet.
