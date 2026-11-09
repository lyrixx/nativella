# Secrets

`Nativella\Secrets` keeps tokens and passwords in the system keychain, which encrypts them: Secret Service (GNOME Keyring, KWallet) on Linux, the Keychain on macOS, the Credential Manager on Windows. Use it for what must not lie in clear in the database or the [settings](settings.md), such as an API token or an account password.

```php
$secrets->set('github-token', $token);

$token = $secrets->get('github-token'); // null when there is none

$secrets->delete('github-token');
```

| Method | |
|---|---|
| `get(string $key): ?string` | The secret, null when there is none. |
| `set(string $key, string $value): void` | Creates or replaces it. |
| `delete(string $key): void` | Deletes it. Nothing happens if there is none. |

An application's secrets are filed under its [identifier](configuration.md#identifier), so that two applications do not mix theirs up. The keychain keeps them encrypted on disk and away from the other users, but not from the user's other programs. On Linux and Windows, any of those programs can read them; macOS asks the user first. On Linux, the keychain must be unlocked, which a desktop session does at login; otherwise the system asks the user for the keychain password first.

`Secrets` is desktop only. On a phone, it throws a `NativeCallException`.

## Secrets and the installation secret

`Nativella\Runtime::secret()` is something else: a random key that Nativella creates for each installation, for the framework to sign and encrypt with (Symfony's `APP_SECRET`, Laravel's `APP_KEY`). See [Configuration](configuration.md#what-php-sees).
