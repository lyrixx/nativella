<?php

namespace Nativella\Website;

/**
 * The few file operations the build needs, failing loudly.
 */
final class Filesystem
{
    private function __construct()
    {
    }

    public static function read(string $file): string
    {
        $content = @file_get_contents($file);
        if (false === $content) {
            throw new \RuntimeException(\sprintf('Cannot read %s.', $file));
        }

        return $content;
    }

    public static function write(string $file, string $content): void
    {
        $dir = \dirname($file);
        if (!is_dir($dir) && !@mkdir($dir, 0o777, true) && !is_dir($dir)) {
            throw new \RuntimeException(\sprintf('Cannot create the directory %s.', $dir));
        }
        if (false === @file_put_contents($file, $content)) {
            throw new \RuntimeException(\sprintf('Cannot write %s.', $file));
        }
    }

    public static function copy(string $from, string $to): void
    {
        $files = new \RecursiveIteratorIterator(new \RecursiveDirectoryIterator($from, \FilesystemIterator::SKIP_DOTS));

        /** @var \SplFileInfo $file */
        foreach ($files as $file) {
            self::write($to.substr($file->getPathname(), \strlen($from)), self::read($file->getPathname()));
        }
    }

    /**
     * Removes a directory and everything in it, if it exists.
     */
    public static function remove(string $dir): void
    {
        if (!is_dir($dir)) {
            return;
        }

        $entries = new \RecursiveIteratorIterator(
            new \RecursiveDirectoryIterator($dir, \FilesystemIterator::SKIP_DOTS),
            \RecursiveIteratorIterator::CHILD_FIRST,
        );

        foreach ($entries as $entry) {
            /** @var \SplFileInfo $entry */
            $removed = $entry->isDir() && !$entry->isLink() ? @rmdir($entry->getPathname()) : @unlink($entry->getPathname());
            if (!$removed) {
                throw new \RuntimeException(\sprintf('Cannot remove %s.', $entry->getPathname()));
            }
        }

        if (!@rmdir($dir)) {
            throw new \RuntimeException(\sprintf('Cannot remove %s.', $dir));
        }
    }
}
