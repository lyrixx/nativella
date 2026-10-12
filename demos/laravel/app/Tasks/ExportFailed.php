<?php

namespace App\Tasks;

final class ExportFailed extends \RuntimeException
{
    public function __construct(
        public readonly string $path,
        \Throwable $previous,
    ) {
        parent::__construct(\sprintf('The tasks could not be written to "%s".', $path), 0, $previous);
    }
}
