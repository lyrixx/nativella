<?php

// The templates of the projects: not those of their vendor/
$directories = glob(__DIR__.'/{demos/*,packages/*,website}/templates', \GLOB_BRACE | \GLOB_ONLYDIR) ?: [];

$finder = TwigCsFixer\File\Finder::create()->in($directories);

return new TwigCsFixer\Config\Config()
    ->setFinder($finder)
;
