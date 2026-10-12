<?php

namespace App\Http\Controllers;

use App\Desktop\Shortcut;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\View\View;
use Nativella\App;
use Nativella\Laravel\Facades\Nativella;
use Nativella\Runtime;
use Nativella\Settings;

final readonly class SettingsController
{
    private const string OPEN_AT_LOGIN = 'open_at_login';

    public function edit(Settings $settings): View
    {
        $mobile = Runtime::isMobile();
        $native = Nativella::isNative();

        return view('settings.edit', [
            'mobile' => $mobile,
            'native' => $native,
            'openAtLogin' => $native && !$mobile && $settings->get(self::OPEN_AT_LOGIN, false),
            'shortcut' => Shortcut::ShowWindow,
        ]);
    }

    public function update(Request $request, App $app, Settings $settings): RedirectResponse
    {
        // Phones and tablets start no application at login.
        abort_if(Runtime::isMobile(), 404);

        $request->validate([
            self::OPEN_AT_LOGIN => ['required', 'boolean'],
        ]);
        $openAtLogin = $request->boolean(self::OPEN_AT_LOGIN);

        if ($openAtLogin) {
            $app->enableAutostart();
        } else {
            $app->disableAutostart();
        }
        $settings->set(self::OPEN_AT_LOGIN, $openAtLogin);

        return to_route('settings.edit')->with('status', $openAtLogin ? 'Tasks will open at login.' : 'Tasks will no longer open at login.');
    }
}
