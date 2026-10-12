<?php

namespace App\View;

use App\Http\Response;
use Nativella\Runtime;

/**
 * Renders the PHP templates of templates/. They run in the scope of this
 * class: they escape with $this->e(), and see their parameters as variables,
 * with $isNative and $isMobile on top.
 */
final readonly class View
{
    public function __construct(
        /** Of the request, for the navigation. */
        private string $currentPath,
        private Flash $flash,
        private string $templateDir,
    ) {
    }

    /**
     * $template in the layout of every page.
     *
     * @param array<string, mixed> $parameters
     */
    public function page(string $title, string $template, array $parameters = [], int $status = 200): Response
    {
        $parameters['isMobile'] = Runtime::isMobile();
        $parameters['isNative'] = Runtime::isNative();

        return Response::html($this->render('layout', [
            'content' => $this->render($template, $parameters),
            'currentPath' => $this->currentPath,
            'flashes' => $this->flash->consume(),
            'isMobile' => $parameters['isMobile'],
            'isNative' => $parameters['isNative'],
            'title' => $title,
        ]), $status);
    }

    /**
     * @param array<string, mixed> $parameters
     */
    public function render(string $template, array $parameters = []): string
    {
        ob_start();
        try {
            $this->include($this->templateDir.'/'.$template.'.php', $parameters);

            return (string) ob_get_contents();
        } finally {
            ob_end_clean();
        }
    }

    /**
     * Escapes text for HTML, in an element or an attribute.
     */
    public function e(string|int $value): string
    {
        return htmlspecialchars((string) $value, \ENT_QUOTES | \ENT_SUBSTITUTE | \ENT_HTML5, 'UTF-8');
    }

    /**
     * @param array<string, mixed> $parameters
     */
    private function include(string $file, array $parameters): void
    {
        extract($parameters, \EXTR_SKIP);

        require $file;
    }
}
