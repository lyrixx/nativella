# Shell

`Nativella\Shell` hands URLs and files over to the applications of the system.

```php
$shell->openUrl('https://github.com/lyrixx/nativella/issues');
$shell->openPath('/home/me/Documents/report.pdf');
$shell->showInFolder('/home/me/Documents/report.pdf');
$shell->trash('/home/me/Documents/old-report.pdf');
```

| Method | |
|---|---|
| `openUrl(string $url): void` | Opens a URL in the default browser, or the mail client for `mailto:`. |
| `openPath(string $path): void` | Opens a file or a directory with its default application. |
| `showInFolder(string $path): void` | Opens the file manager on the directory of `$path`, with `$path` selected. |
| `trash(string $path): void` | Moves a file or a directory to the trash, where the user can restore it. Desktop only. |

## Links to other sites

A plain link to another website already opens in the browser of the system, not in the window: see [Pages and JavaScript](javascript.md#links-and-absolute-urls). `openUrl()` is for the other cases, when PHP decides what to open:

```php
use Nativella\Event\MenuItemClicked;

if ($event instanceof MenuItemClicked && 'help.docs' === $event->id) {
    $shell->openUrl('https://github.com/lyrixx/nativella/tree/main/docs');
}
```

`openUrl()` opens whatever it is given: only hand it URLs you trust, or that the user typed.
