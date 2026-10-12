<?php

namespace Nativella\Laravel\Tests;

final class OutsideNativellaTest extends TestCase
{
    private const string KEY = 'base64:2fl+Ktvkfl+Fuz4Qp/A75G2RTiWVA/ZoKZvp6fiiM10=';

    public function testTheDatabaseConfigurationApplies(): void
    {
        $this->assertSame('testing', config('database.default'));
        $this->assertSame(database_path('database.sqlite'), config('database.connections.sqlite.database'));
    }

    public function testTheApplicationKeyApplies(): void
    {
        $this->assertSame(self::KEY, config('app.key'));
    }

    protected function defineEnvironment($app): void
    {
        $app['config']->set('app.key', self::KEY);
    }
}
