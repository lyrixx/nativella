# Clipboard

`Nativella\Clipboard` reads and writes the clipboard of the system, from PHP.

```php
$clipboard->writeText($snippet->code);

$clipboard->writeHtml('<b>Groceries</b>: eggs, flour', altText: 'Groceries: eggs, flour');

$pasted = $clipboard->readText(); // null when the clipboard holds no text

$clipboard->writeImage(file_get_contents('chart.png'));
$png = $clipboard->readImage(); // PNG bytes, null when the clipboard holds no image
```

| Method | |
|---|---|
| `readText(): ?string` | The text of the clipboard. Null when it holds none: it is empty, or holds an image. |
| `writeText(string $text): void` | Replaces the content of the clipboard with text. |
| `writeHtml(string $html, ?string $altText = null): void` | Replaces it with HTML. `$altText` is what applications that do not read HTML paste. |
| `readImage(): ?string` | The image of the clipboard, as the bytes of a PNG file. Null when it holds none. |
| `writeImage(string $png): void` | Replaces the content of the clipboard with an image, given as the bytes of a PNG file. |
| `clear(): void` | Empties the clipboard. |

Copy and paste inside the page work as in any browser, with the keyboard and the [Edit menu](menus.md#roles). `Clipboard` is for PHP: copy a result the user did not select, or read what they copied elsewhere.

## Example: a copy button

```html
<form method="post" action="/snippets/42/copy">
    <button>Copy to clipboard</button>
</form>
```

```php
// POST /snippets/{id}/copy
$clipboard->writeText($snippet->code);
$notification->send('Copied', $snippet->title);

header('Location: /snippets/'.$snippet->id, true, 303);
```
