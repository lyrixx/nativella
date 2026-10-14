<?php

namespace Nativella\Website;

use Tempest\Highlight\Highlighter;
use Twig\Environment;
use Twig\Loader\FilesystemLoader;

/**
 * Builds the landing page, from the Twig templates of templates/. Zensical
 * builds the documentation under it, into docs/: see zensical.yml.
 */
final readonly class Site
{
    public const string REPOSITORY = 'https://github.com/lyrixx/nativella';

    public function __construct(
        private string $projectDir,
        private string $websiteDir,
        private string $outputDir,
        /** Where the site is published, with a trailing slash. */
        private string $siteUrl,
    ) {
    }

    public function build(): void
    {
        $twig = new Environment(new FilesystemLoader($this->websiteDir.'/templates'), [
            'strict_variables' => true,
        ]);
        $twig->addExtension(new TwigExtension($this->websiteDir.'/assets', new Highlighter()));
        $twig->addGlobal('site', new SiteInfo(
            repository: self::REPOSITORY,
            url: $this->siteUrl,
            version: $this->version(),
        ));

        Filesystem::remove($this->outputDir);
        Filesystem::copy($this->websiteDir.'/assets', $this->outputDir.'/assets');
        // GitHub Pages: serve the files as they are.
        Filesystem::write($this->outputDir.'/.nojekyll', '');
        Filesystem::write($this->outputDir.'/index.html', $twig->render('home.html.twig', [
            'architecture' => $this->architectureDiagram(),
        ]));
    }

    /**
     * The diagram of docs/how-it-works.md, inlined in the landing page so that
     * it follows the theme of the page.
     */
    private function architectureDiagram(): string
    {
        $svg = Filesystem::read($this->projectDir.'/docs/images/architecture.svg');

        return trim(Regex::replace('/^<\?xml[^>]*>\s*/', '', $svg));
    }

    /**
     * The version of the workspace, from the root Cargo.toml.
     */
    private function version(): string
    {
        $manifest = Filesystem::read($this->projectDir.'/Cargo.toml');
        if (!preg_match('/^\[workspace\.package\][^\[]*^version\s*=\s*"([^"]+)"/ms', $manifest, $matches)) {
            throw new \RuntimeException('The root Cargo.toml has no [workspace.package] version.');
        }

        return $matches[1];
    }
}
