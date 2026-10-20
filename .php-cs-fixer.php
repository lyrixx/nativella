<?php

// The code of the projects and the Castor tasks. The skeleton directories of
// the frameworks (config/, Laravel's app/, database/, routes/...) are left out,
// and so are PHP templates: the Symfony style would turn their short echo tags
// into echo statements, and their alternative syntax (endif...) into braces.
$directories = glob(__DIR__.'/{.castor,demos/*/src,demos/*/tests,packages/*/src,packages/*/tests,website}', \GLOB_BRACE | \GLOB_ONLYDIR) ?: [];

$finder = PhpCsFixer\Finder::create()
    ->ignoreVCSIgnored(true)
    ->in($directories)
    ->exclude(['cache', 'dist', 'storage', 'templates', 'var', 'vendor'])
    ->append([
        __DIR__.'/castor.php',
        __DIR__.'/.twig-cs-fixer.php',
        __FILE__,
        // PHP scripts without the .php extension
        ...glob(__DIR__.'/website/bin/*') ?: [],
    ])
;

return new PhpCsFixer\Config()
    ->setUnsupportedPhpVersionAllowed(true)
    ->setRiskyAllowed(true)
    ->setRules([
        '@PHP8x4Migration' => true,
        '@Symfony' => true,
        '@Symfony:risky' => true,
    ])
    ->setFinder($finder)
;
