<?php

namespace App\Native;

use Nativella\Dialog\MessageKind;
use Nativella\Nativella;
use Nativella\Runtime;

/**
 * Asks before a snippet is deleted: on a phone with a fingerprint or a face
 * enrolled, the user proves who they are; elsewhere, a dialog asks them.
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
        if (Runtime::isMobile() && $this->native->biometrics->isAvailable()) {
            return $this->native->biometrics->authenticate($question, self::TITLE);
        }

        return $this->native->dialog->confirm($question, self::TITLE, MessageKind::Warning, 'Delete', 'Cancel');
    }
}
