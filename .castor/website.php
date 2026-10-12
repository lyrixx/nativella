<?php

namespace website;

use Castor\Attribute\AsTask;
use project\Project;

use function Castor\io;
use function project\run_or_fail;
use function qa\install_mkdocs;

/**
 * The landing page, by a PHP script, then the documentation under it, in
 * docs/, by MkDocs.
 *
 * NATIVELLA_SITE_URL, passed to the builder when set, is where the site is
 * published, for its few absolute URLs; the frontend serves it on the root
 * domain whatever the value.
 */
#[AsTask(description: 'Builds the website and the documentation into website/dist/')]
function build(): void
{
    io()->title('Building the website');

    $website = new Project('website');

    // Cheap when installed already, and the deployment needs nothing else
    run_or_fail($website, ['composer', 'install', '-n', '--prefer-dist', '--optimize-autoloader']);
    install_mkdocs();

    // The landing page first: it empties dist/
    run_or_fail($website, ['php', 'bin/build']);
    run_or_fail($website, ['mkdocs', 'build']);
}
