<?php

namespace Nativella\Laravel\Tests;

use Nativella\Laravel\NativellaServiceProvider;
use Orchestra\Testbench\TestCase as BaseTestCase;

abstract class TestCase extends BaseTestCase
{
    protected function getPackageProviders($app): array
    {
        return [NativellaServiceProvider::class];
    }
}
