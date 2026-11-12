<?php

namespace App\Native;

use Nativella\Dialog\MessageKind;
use Nativella\Exception\NativeCallException;
use Nativella\Nativella;
use Nativella\Runtime;

/**
 * Asks before a snippet is deleted: on a phone, the user proves who they are,
 * with their fingerprint, their face or the PIN of the phone; elsewhere, a
 * dialog asks them.
 */
final readonly class DeleteConfirmation
{
    private const string TITLE = 'Delete the snippet';

    public function __construct(
        private Nativella $native,
    ) {
    }

    public function confirm(string $question): bool
    {
        if (Runtime::isMobile()) {
            try {
                // Whatever Biometrics::isAvailable() says: without a fingerprint
                // or a face enrolled, the phone asks for its PIN.
                return $this->native->biometrics->authenticate($question, self::TITLE);
            } catch (NativeCallException $e) {
                // The phone has no lock at all: nobody to tell from its owner.
                error_log(\sprintf('Warning: the phone cannot tell who holds it, a dialog asks instead. %s', $e->getMessage()));
            }
        }

        return $this->native->dialog->confirm($question, self::TITLE, MessageKind::Warning, 'Delete', 'Cancel');
    }
}
