<?php

namespace website;

use Castor\Attribute\AsTask;
use project\Project;

use function Castor\io;
use function docker\builder_path;
use function docker\run_in_builder;
use function project\run_or_fail;
use function qa\install_zensical;

/**
 * The landing page, by a PHP script, which stages the documents of docs/ too,
 * then the documentation under it, in docs/, by Zensical.
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
    install_zensical();

    // The landing page first: it empties dist/
    run_or_fail($website, ['php', 'bin/build']);

    // Where the documentation is published, under the landing page: see
    // zensical.yml, which has the default
    $siteUrl = getenv('NATIVELLA_SITE_URL');
    $environment = $siteUrl ? ['NATIVELLA_DOCS_URL' => rtrim($siteUrl, '/').'/docs/'] : [];
    // By its path: on the host, tools/bin is not in PATH
    run_in_builder([builder_path('tools/zensical/.venv/bin/zensical'), 'build', '--config-file', 'zensical.yml'], builder_path('website'), $environment);
}
