<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Support\Facades\Route;
use Nativella\Laravel\Facades\Nativella;
use Nativella\Laravel\Http\Middleware\EnsureNative;

final class EnsureNativeTest extends TestCase
{
    protected function defineRoutes($router): void
    {
        Route::get('/export', static fn (): string => 'Exported.')->middleware(EnsureNative::class);
    }

    public function testTheRouteDoesNotExistOutsideNativella(): void
    {
        $this->get('/export')->assertNotFound();
    }

    public function testTheRouteWorksInNativella(): void
    {
        Nativella::fake();

        $this->get('/export')->assertOk()->assertSee('Exported.');
    }
}
