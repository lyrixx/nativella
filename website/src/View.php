<?php

namespace Nativella\Website;

/**
 * What a template sees as $this: helpers to escape, link and highlight.
 *
 * Every URL is relative to the page, so that the site works at the root of a
 * domain as well as under a path (https://lyrixx.github.io/nativella/).
 */
final class View
{
    /** @var array{0: string, 1: array<string, mixed>}|null */
    private ?array $layout = null;

    public function __construct(
        private readonly Renderer $renderer,
        /** The page being rendered, from the root of the site: "index.html". */
        public readonly string $path,
        public readonly SiteInfo $site,
    ) {
    }

    /**
     * @param array<string, mixed> $variables
     */
    public function render(string $template, array $variables = []): string
    {
        $content = $this->include($template, $variables);

        if (null === $this->layout) {
            return $content;
        }

        [$layout, $layoutVariables] = $this->layout;
        $this->layout = null;

        return $this->render($layout, ['content' => $content] + $layoutVariables + $variables);
    }

    /**
     * Wraps the current template in $layout, which gets its output as $content.
     *
     * @param array<string, mixed> $variables
     */
    public function extend(string $layout, array $variables = []): void
    {
        $this->layout = [$layout, $variables];
    }

    /**
     * @param array<string, mixed> $variables
     */
    public function partial(string $template, array $variables = []): string
    {
        return new self($this->renderer, $this->path, $this->site)->include($template, $variables);
    }

    public function e(string|int|float|null $value): string
    {
        return htmlspecialchars((string) $value, \ENT_QUOTES | \ENT_SUBSTITUTE, 'UTF-8');
    }

    /**
     * The relative URL of $path, given from the root of the site.
     */
    public function url(string $path): string
    {
        $from = explode('/', $this->path);
        array_pop($from);
        $to = explode('/', $path);

        // Leave out the directories both paths share.
        while ([] !== $from && \count($to) > 1 && $from[0] === $to[0]) {
            array_shift($from);
            array_shift($to);
        }

        return str_repeat('../', \count($from)).implode('/', $to);
    }

    public function asset(string $asset): string
    {
        return $this->url('assets/'.$asset).'?v='.$this->renderer->assetVersion($asset);
    }

    /**
     * The absolute URL of $path, for the few places that need one (Open Graph).
     */
    public function absolute(string $path = ''): string
    {
        return $this->site->url.$path;
    }

    public function code(string $language, string $code, ?string $title = null): string
    {
        return $this->renderer->code($language, $code, $title);
    }

    /**
     * @param array<string, mixed> $variables
     */
    private function include(string $template, array $variables): string
    {
        $file = $this->renderer->template($template);
        $render = function (string $__file, array $__variables): void {
            extract($__variables, \EXTR_SKIP);
            require $__file;
        };

        ob_start();
        try {
            $render($file, $variables);
        } finally {
            $output = (string) ob_get_clean();
        }

        return $output;
    }
}
