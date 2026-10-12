<?php

use Illuminate\Support\Facades\Route;
use Nativella\Laravel\Http\NativeEventController;
use Nativella\Runtime;

// Outside the "web" middleware group: native events need neither a session
// nor a CSRF token.
Route::post(Runtime::EVENT_PATH, NativeEventController::class)->name('nativella.event');
