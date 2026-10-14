<?php

namespace Nativella\Website;

/**
 * Stages the documents of docs/ for Zensical, which builds the documentation
 * from there: see zensical.yml.
 *
 * The documents are written for GitHub first, and read well there: the
 * navigation is the list of docs/README.md, and links to the rest of the
 * repository are relative. The staged copy makes them a site.
 */
final readonly class Documentation
{
    /**
     * docs/README.md lists the pages by group, one per line:
     *
     *     ## Native APIs
     *
     *     - [Windows](windows.md): open, move, resize and close windows.
     */
    private const string GROUP = '/^## (.+)$/';
    private const string PAGE = '/^- \[([^\]]+)\]\(([\w-]+\.md)\)/';

    /** A link out of docs/: [the runtime](../crates/runtime#readme) */
    private const string REPOSITORY_LINK = '/\]\((\.\.\/[^)#\s]+)(#[^)\s]*)?\)/';

    public function __construct(
        private string $projectDir,
        private string $outputDir,
    ) {
    }

    public function stage(): void
    {
        $docsDir = $this->projectDir.'/docs';

        Filesystem::remove($this->outputDir);
        Filesystem::copy($docsDir, $this->outputDir);

        foreach (glob($docsDir.'/*.md') ?: [] as $document) {
            Filesystem::write($this->outputDir.'/'.basename($document), $this->linkToGitHub($document));
        }

        // YAML, as JSON is
        Filesystem::write(
            $this->outputDir.'/.nav.yml',
            "# Written by bin/build, from docs/README.md\n".json_encode(['nav' => $this->navigation($docsDir)], \JSON_PRETTY_PRINT | \JSON_THROW_ON_ERROR | \JSON_UNESCAPED_SLASHES | \JSON_UNESCAPED_UNICODE)."\n",
        );
    }

    /**
     * The document, its links to the rest of the repository leading to
     * GitHub: they are relative, for GitHub, but the site only holds docs/.
     */
    private function linkToGitHub(string $document): string
    {
        // As realpath() gives the targets
        $root = realpath($this->projectDir) ?: throw new \RuntimeException(\sprintf('Cannot find %s.', $this->projectDir));
        $docsDir = $root.'/docs';
        $lines = [];
        $inCode = false;

        foreach (explode("\n", Filesystem::read($document)) as $line) {
            if (str_starts_with($line, '```')) {
                $inCode = !$inCode;
            } elseif (!$inCode) {
                $line = Regex::replaceCallback(self::REPOSITORY_LINK, static function (array $link) use ($document, $docsDir, $root): string {
                    $target = realpath(\dirname($document).'/'.$link[1]);
                    if (false === $target || !str_starts_with($target, $root.'/')) {
                        throw new \RuntimeException(\sprintf('docs/%s links to %s, which is not a file or a directory of the repository.', basename($document), $link[1]));
                    }
                    if (str_starts_with($target, $docsDir.'/')) {
                        return $link[0];
                    }

                    return \sprintf('](%s/%s/main/%s%s)', Site::REPOSITORY, is_dir($target) ? 'tree' : 'blob', substr($target, \strlen($root) + 1), $link[2] ?? '');
                }, $line);
            }
            $lines[] = $line;
        }

        return implode("\n", $lines);
    }

    /**
     * The navigation of docs/README.md: its title first, as "Overview", then
     * one section per group. Every other document must be in it.
     *
     * @return list<array<string, string|list<array<string, string>>>>
     */
    private function navigation(string $docsDir): array
    {
        $nav = [['Overview' => 'README.md']];
        $listed = [];
        $group = null;
        $pages = [];

        foreach (explode("\n", Filesystem::read($docsDir.'/README.md')) as $line) {
            if ($match = Regex::match(self::GROUP, $line)) {
                if (null !== $group) {
                    $nav[] = [$group => $this->pagesOf($group, $pages)];
                }
                $group = trim($match[1]);
                $pages = [];
            } elseif ($match = Regex::match(self::PAGE, $line)) {
                if (null === $group) {
                    throw new \RuntimeException(\sprintf('docs/README.md lists %s before its first "## Group".', $match[2]));
                }
                if (!is_file($docsDir.'/'.$match[2])) {
                    throw new \RuntimeException(\sprintf('docs/README.md lists %s, which does not exist.', $match[2]));
                }
                $pages[] = [$match[1] => $match[2]];
                $listed[] = $match[2];
            }
        }
        if (null !== $group) {
            $nav[] = [$group => $this->pagesOf($group, $pages)];
        }

        foreach (glob($docsDir.'/*.md') ?: [] as $document) {
            if ('README.md' !== basename($document) && !\in_array(basename($document), $listed, true)) {
                throw new \RuntimeException(\sprintf('docs/README.md, the navigation, does not list docs/%s.', basename($document)));
            }
        }

        return $nav;
    }

    /**
     * @param list<array<string, string>> $pages
     *
     * @return list<array<string, string>>
     */
    private function pagesOf(string $group, array $pages): array
    {
        if ([] === $pages) {
            throw new \RuntimeException(\sprintf('The "%s" group of docs/README.md lists no page.', $group));
        }

        return $pages;
    }
}
