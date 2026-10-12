<x-layout title="Settings">
    @if ($mobile)
        <p class="empty">There is nothing to set on this device.</p>
    @else
        <form class="settings" method="POST" action="{{ route('settings.update') }}">
            @csrf
            @method('PUT')
            <label class="setting">
                <span>
                    <strong>Open at login</strong>
                    <small>Start Tasks when you log in to this computer.</small>
                </span>
                <input type="hidden" name="open_at_login" value="0">
                <input type="checkbox" name="open_at_login" value="1" role="switch" @checked($openAtLogin) @disabled(! $native)>
            </label>
            <div class="setting">
                <span>
                    <strong>Bring Tasks to the front</strong>
                    <small>From any application, even with the window closed.</small>
                </span>
                <kbd>{{ $shortcut->label() }}</kbd>
            </div>

            @if ($native)
                <div class="actions">
                    <button type="submit" class="button primary">Save</button>
                </div>
            @else
                <p class="hint">These settings only exist in the desktop application.</p>
            @endif
        </form>
    @endif
</x-layout>
