@props(['title'])
<!DOCTYPE html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}" @if (\Nativella\Runtime::isMobile()) data-mobile @endif>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="color-scheme" content="light dark">
    <title>{{ $title }} · {{ config('app.name') }}</title>
    <link rel="stylesheet" href="{{ asset('app.css') }}">
</head>
<body>
    <header class="toolbar">
        <h1>{{ $title }}</h1>
        <nav class="segmented" aria-label="Sections">
            <a href="{{ route('tasks.index') }}" @class(['active' => request()->routeIs('tasks.*')])>Tasks</a>
            <a href="{{ route('settings.edit') }}" @class(['active' => request()->routeIs('settings.*')])>Settings</a>
        </nav>
    </header>

    <main>
        @session('status')
            <p class="banner" role="status">{{ $value }}</p>
        @endsession
        @session('error')
            <p class="banner error" role="alert">{{ $value }}</p>
        @endsession

        {{ $slot }}
    </main>
</body>
</html>
