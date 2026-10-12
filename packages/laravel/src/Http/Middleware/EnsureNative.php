<?php

namespace Nativella\Laravel\Http\Middleware;

use Illuminate\Http\Request;
use Nativella\Laravel\Facades\Nativella;
use Symfony\Component\HttpFoundation\Response;

/**
 * Hides a route outside Nativella (404), for what only the desktop
 * application can do:
 *
 *     Route::post('/export', ExportController::class)->middleware(EnsureNative::class);
 */
final readonly class EnsureNative
{
    /**
     * @param \Closure(Request): Response $next
     */
    public function handle(Request $request, \Closure $next): Response
    {
        abort_unless(Nativella::isNative(), Response::HTTP_NOT_FOUND);

        return $next($request);
    }
}
