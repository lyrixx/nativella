<?php

namespace App\Import;

final readonly class RejectedFile
{
    public function __construct(
        /** The base name of the file. */
        public string $name,
        /** A full sentence. */
        public string $reason,
    ) {
    }
}
