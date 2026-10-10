<?php

namespace App\Native;

use Symfony\Component\Validator\Constraints as Assert;

final readonly class WindowTitle
{
    public function __construct(
        #[Assert\NotBlank]
        #[Assert\Length(max: 100)]
        public string $title,
    ) {
    }
}
