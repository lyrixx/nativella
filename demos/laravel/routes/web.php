<?php

use App\Http\Controllers\CopyTasksController;
use App\Http\Controllers\ExportTasksController;
use App\Http\Controllers\SettingsController;
use App\Http\Controllers\TaskController;
use Illuminate\Support\Facades\Route;
use Nativella\Laravel\Http\Middleware\EnsureNative;

Route::get('/', [TaskController::class, 'index'])->name('tasks.index');
Route::post('/tasks', [TaskController::class, 'store'])->name('tasks.store');
Route::post('/tasks/{task}/complete', [TaskController::class, 'complete'])->name('tasks.complete');
Route::post('/tasks/{task}/reopen', [TaskController::class, 'reopen'])->name('tasks.reopen');
Route::delete('/tasks/{task}', [TaskController::class, 'destroy'])->name('tasks.destroy');
Route::get('/settings', [SettingsController::class, 'edit'])->name('settings.edit');

// What only the desktop application can do.
Route::middleware(EnsureNative::class)->group(function (): void {
    Route::post('/tasks/copy', CopyTasksController::class)->name('tasks.copy');
    Route::post('/tasks/export', ExportTasksController::class)->name('tasks.export');
    Route::put('/settings', [SettingsController::class, 'update'])->name('settings.update');
});
