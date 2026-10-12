# Settings

`Nativella\Settings` is a small key-value store for preferences: the theme, the last folder opened, a shortcut the user picked. The runtime keeps it as `settings.json` in the [data directory](configuration.md#the-data-directory), so it survives restarts and updates.

```php
$settings->set('editor.fontSize', 15);
$settings->set('recent', ['/home/me/notes.md', '/home/me/todo.md']);

$size = $settings->get('editor.fontSize', 14);
```

Values are anything JSON can hold: strings, numbers, booleans, null, and arrays of those. Application data belongs in the [database](database.md); settings are for the handful of values around it.

| Method | |
|---|---|
| `get(string $key, mixed $default = null): mixed` | The value of `$key`, or `$default` when there is none. |
| `has(string $key): bool` | Whether `$key` is set, even to null. |
| `set(string $key, mixed $value): void` | |
| `forget(string $key): void` | Removes `$key`. |
| `all(): array` | Every setting, by key. |
| `clear(): void` | Removes every setting. |

Keys are plain strings: the dot in `editor.fontSize` is a convention, not a path. A key set to null is still set: `get()` returns null, not the default, and `has()` returns true.

Every change is written to disk right away, to a temporary file then renamed, so that a crash never leaves half a file behind.

## Settings or database?

Settings are global to the application and read one key at a time. They suit preferences the user could reset without losing work. Anything you would list, search, or migrate belongs in the database.

Settings are stored in clear, in a JSON file: keep tokens and passwords in the [keychain of the system](secrets.md) instead.
