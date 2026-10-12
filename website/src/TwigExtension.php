<?php

namespace Nativella\Website;

use Tempest\Highlight\Highlighter;
use Twig\Extension\AbstractExtension;
use Twig\TwigFilter;
use Twig\TwigFunction;

/**
 * What the templates need beyond Twig: versioned assets, highlighted code.
 */
final class TwigExtension extends AbstractExtension
{
    public function __construct(
        private readonly string $assetDir,
        private readonly Highlighter $highlighter,
    ) {
    }

    public function getFunctions(): array
    {
        return [
            new TwigFunction('asset', $this->asset(...)),
        ];
    }

    public function getFilters(): array
    {
        return [
            new TwigFilter('highlight', $this->highlight(...), ['is_safe' => ['html']]),
        ];
    }

    /**
     * The URL of an asset, with a short hash of its content, to bust caches
     * when it changes.
     */
    public function asset(string $asset): string
    {
        $file = $this->assetDir.'/'.$asset;
        if (!is_file($file)) {
            throw new \LogicException(\sprintf('The asset "%s" does not exist.', $asset));
        }

        $hash = hash_file('xxh3', $file);
        if (false === $hash) {
            throw new \RuntimeException(\sprintf('Cannot read the asset "%s".', $asset));
        }

        return 'assets/'.$asset.'?v='.substr($hash, 0, 8);
    }

    /**
     * A code block, from the code a template gives between
     * `{% apply highlight('php', 'src/Kernel.php') %}` and `{% endapply %}`:
     * without the indentation of the template.
     */
    public function highlight(string $code, string $language, ?string $title = null): string
    {
        // Up to the indentation of {% endapply %}
        $code = rtrim($code);
        preg_match_all('/^( *)\S/m', $code, $indentations);
        $indentation = min(array_map(strlen(...), $indentations[1]) ?: [0]);

        return CodeBlock::render($this->highlighter, $language, Regex::replace("/^ {{$indentation}}/m", '', $code), $title);
    }
}
