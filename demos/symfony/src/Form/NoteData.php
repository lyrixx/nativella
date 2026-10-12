<?php

namespace App\Form;

use App\Entity\Note;
use Symfony\Component\Validator\Constraints as Assert;

/**
 * What the note form edits.
 */
final class NoteData
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: Note::TITLE_MAX_LENGTH)]
        public string $title = '',
        public string $body = '',
    ) {
    }

    public static function fromNote(Note $note): self
    {
        return new self($note->getTitle(), $note->getBody());
    }
}
