<?php

namespace Nativella\Website;

use Tempest\Highlight\Highlighter;

/**
 * A highlighted code block, with a caption: its file name when there is one,
 * its language otherwise. The JavaScript of the site adds a copy button.
 */
final class CodeBlock
{
    private const array LANGUAGES = [
        'bash' => 'Shell',
        'blade' => 'Blade',
        'css' => 'CSS',
        'diff' => 'Diff',
        'dotenv' => '.env',
        'env' => '.env',
        'html' => 'HTML',
        'ini' => 'INI',
        'js' => 'JavaScript',
        'javascript' => 'JavaScript',
        'json' => 'JSON',
        'php' => 'PHP',
        'rust' => 'Rust',
        'sh' => 'Shell',
        'shell' => 'Shell',
        'sql' => 'SQL',
        'twig' => 'Twig',
        'xml' => 'XML',
        'yaml' => 'YAML',
    ];

    private function __construct()
    {
    }

    public static function render(Highlighter $highlighter, string $language, string $code, ?string $title = null): string
    {
        $language = strtolower($language);
        $code = rtrim($code, "\n");
        $label = self::LANGUAGES[$language] ?? null;

        $caption = '';
        if (null !== $title || null !== $label) {
            $caption = \sprintf(
                '<figcaption class="code-caption">%s%s</figcaption>',
                null !== $title ? '<span class="code-title">'.htmlspecialchars($title).'</span>' : '',
                null !== $label ? '<span class="code-language">'.$label.'</span>' : '',
            );
        }

        return \sprintf(
            '<figure class="code">%s<pre class="code-body" data-lang="%s"><code>%s</code></pre></figure>'."\n",
            $caption,
            htmlspecialchars($language),
            $highlighter->parse($code, $language),
        );
    }
}
