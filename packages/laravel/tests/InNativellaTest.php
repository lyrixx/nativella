<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Support\Facades\Blade;
use Nativella\Laravel\Facades\Nativella;
use Nativella\Laravel\Facades\Notification;
use PHPUnit\Framework\Attributes\RunTestsInSeparateProcesses;

/**
 * As PHP runs in Nativella: nativella_call() exists. A function cannot be
 * removed once defined, hence the separate processes.
 */
#[RunTestsInSeparateProcesses]
final class InNativellaTest extends TestCase
{
    protected function setUp(): void
    {
        require_once __DIR__.'/Fixtures/nativella_call.php';
        $GLOBALS['nativella_calls'] = [];

        parent::setUp();
    }

    public function testTheApisCallTheRuntime(): void
    {
        $this->assertTrue(Nativella::isNative());

        Notification::send('Saved');

        $this->assertSame([['method' => 'notification.send', 'payload' => ['title' => 'Saved', 'body' => null, 'reference' => null]]], $GLOBALS['nativella_calls']);
    }

    public function testTheNativeDirectiveShowsNativeFeatures(): void
    {
        $this->assertSame('desktop', trim(Blade::render('@native desktop @else browser @endnative')));
    }
}
