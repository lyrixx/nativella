<?php

namespace Nativella\Symfony\Tests\Twig;

use Nativella\Client;
use Nativella\Client\ExtensionClient;
use Nativella\Client\FakeClient;
use Nativella\Symfony\Twig\NativellaExtension;
use PHPUnit\Framework\Attributes\BackupGlobals;
use PHPUnit\Framework\TestCase;
use Twig\Environment;
use Twig\Loader\ArrayLoader;

#[BackupGlobals(true)]
final class NativellaExtensionTest extends TestCase
{
    public function testOnAPhone(): void
    {
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        $this->assertSame('native mobile android', self::render(new FakeClient()));
    }

    public function testOnTheDesktop(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'linux';

        $this->assertSame('native desktop linux', self::render(new FakeClient()));
    }

    public function testOutsideNativella(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);

        $this->assertSame('web desktop ', self::render(new ExtensionClient()));
    }

    private static function render(Client $client): string
    {
        $twig = new Environment(new ArrayLoader([
            'page' => '{{ nativella_is_native() ? "native" : "web" }} {{ nativella_is_mobile() ? "mobile" : "desktop" }} {{ nativella_platform() }}',
        ]), ['strict_variables' => true]);
        $twig->addExtension(new NativellaExtension($client));

        return $twig->render('page');
    }
}
