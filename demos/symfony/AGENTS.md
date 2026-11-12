# AGENTS.md

This is the Symfony demo of Nativella, Nativella Notes. The instructions of the repository apply:
read [`AGENTS.md`](../../AGENTS.md) at its root first.

PHP and Composer never run on the host. Everything goes through the `builder` container, with
Castor, from the root of the repository:

```bash
castor builder -- composer require foo/bar -d demos/symfony   # a dependency
castor builder -- demos/symfony/bin/console debug:router      # a console command
castor qa:phpunit --project=demos/symfony                     # the tests
castor qa:phpstan --project=demos/symfony                     # PHPStan
castor qa:cs                                                  # PHP CS Fixer, from tools/php-cs-fixer
castor qa:twig-cs                                             # Twig CS Fixer
castor nativella:run symfony                                  # the demo, as a desktop application
```

## What is specific to the demo

- [`README.md`](README.md) lists the features, the native API each one uses, and how the demo works.
- The packages come from `packages/*` through a Composer path repository: a change to
  `packages/nativella` or `packages/symfony` shows here without an install.
- The native parts live in `src/Native/Desktop.php` (window, menu, tray) and `src/EventListener/`
  (one listener per native event).
- Guard a native feature with `isNative()` of the `Nativella\Nativella` service, or
  `nativella_is_native()` in Twig, and what only computers have with `Runtime::isMobile()`: a native
  route answers 404 where its feature does not exist.
- Schema changes go through Doctrine migrations (`migrations/`), which the bundle runs when the
  application starts. A migration that shipped is never edited.
- Tests extend `App\Tests\NotesTestCase`: the fake client answers the native calls
  (`fakeClient()`), `sendNativeEvent()` posts an event as the runtime does, and `leaveNativella()`
  runs a test as a browser would. A native feature needs a test of both.
