<?php

namespace App\Note\Exception;

use Symfony\Component\Filesystem\Exception\IOException;

final class ExportFailed extends \RuntimeException
{
    public function __construct(
        public readonly string $path,
        IOException $previous,
    ) {
        parent::__construct(\sprintf('The note could not be written to "%s".', $path), 0, $previous);
    }
}
