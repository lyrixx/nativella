<?php

use Doctrine\Deprecations\Deprecation;

require dirname(__DIR__).'/vendor/autoload.php';

// Deprecated DBAL APIs fail the tests (failOnDeprecation): the schema code
// keeps to the current ones.
Deprecation::enableWithTriggerError();
