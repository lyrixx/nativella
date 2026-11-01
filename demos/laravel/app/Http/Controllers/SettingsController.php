<?php

namespace App\Http\Controllers;

use App\Desktop\Shortcut;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\View\View;
use Nativella\App;
use Nativella\Laravel\Facades\Nativella;
use Nativella\Runtime;

final readonly class SettingsController
{
    public function edit(App $app): View
    {
        $mobile = Runtime::isMobile();
        $native = Nativella::isNative();

        return view('settings.edit', [
            // As the system has it: the user may have changed it there.
            'autostart' => $native && !$mobile && $app->isAutostartEnabled(),
            'mobile' => $mobile,
            'native' => $native,
            'shortcut' => Shortcut::ShowWindow,
        ]);
    }

    public function update(Request $request, App $app): RedirectResponse
    {
        // Phones and tablets start no application at login.
        abort_if(Runtime::isMobile(), 404);

        $request->validate([
            'autostart' => ['required', 'boolean'],
        ]);
        $autostart = $request->boolean('autostart');

        if ($autostart) {
            $app->enableAutostart();
        } else {
            $app->disableAutostart();
        }

        return to_route('settings.edit')->with('status', $autostart ? 'Tasks will open at login.' : 'Tasks will no longer open at login.');
    }
}
