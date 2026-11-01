<?php

namespace download;

use function Castor\fs;
use function Castor\http_download;

/**
 * Downloads $url into $file, and removes it unless it has the SHA-256 it was
 * pinned with: the tools the tasks download build the releases.
 */
function verified(string $url, string $file, string $sha256): void
{
    http_download($url, $file);

    $actual = hash_file('sha256', $file);
    if ($sha256 !== $actual) {
        fs()->remove($file);

        throw new \RuntimeException(\sprintf('%s has the SHA-256 %s, not the %s it was pinned with.', $url, $actual, $sha256));
    }
}
