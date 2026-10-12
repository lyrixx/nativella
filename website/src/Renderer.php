<?php

namespace Nativella\Website;

use Tempest\Highlight\Highlighter;

/**
 * Renders the templates of templates/, plain PHP files.
 */
final readonly class Renderer
{
    public function __construct(
        private string $templateDir,
        private string $assetDir,
        private Highlighter $highlighter,
        private SiteInfo $site,
    ) {
    }

    /**
     * @param string               $path      the page being rendered, from the root of the site: "index.html"
     * @param array<string, mixed> $variables
     */
    public function render(string $template, string $path, array $variables = []): string
    {
        return new View($this, $path, $this->site)->render($template, $variables);
    }

    public function template(string $name): string
    {
        $file = $this->templateDir.'/'.$name.'.php';
        if (!is_file($file)) {
            throw new \LogicException(\sprintf('The template "%s" does not exist.', $name));
        }

        return $file;
    }

    /**
     * A short hash of an asset, to bust caches when it changes.
     */
    public function assetVersion(string $asset): string
    {
        $file = $this->assetDir.'/'.$asset;
        if (!is_file($file)) {
            throw new \LogicException(\sprintf('The asset "%s" does not exist.', $asset));
        }

        $hash = hash_file('xxh3', $file);
        if (false === $hash) {
            throw new \RuntimeException(\sprintf('Cannot read the asset "%s".', $asset));
        }

        return substr($hash, 0, 8);
    }

    public function code(string $language, string $code, ?string $title = null): string
    {
        return CodeBlock::render($this->highlighter, $language, $code, $title);
    }
}
