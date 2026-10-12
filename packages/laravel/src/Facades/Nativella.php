<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Contracts\Foundation\Application;
use Illuminate\Support\Facades\Facade;
use Nativella\Client;
use Nativella\Client\FakeClient;
use Nativella\Runtime;

/**
 * Nativella itself: whether the application runs in it, and a fake of it for
 * tests. Its root holds every API, as Nativella\Nativella does.
 *
 * @see \Nativella\Nativella
 */
final class Nativella extends Facade
{
    /**
     * Whether native calls work: the application runs in Nativella, or the
     * tests faked it. Offer native features only then, with @native in Blade.
     */
    public static function isNative(): bool
    {
        return Runtime::isNative() || self::application()->make(Client::class) instanceof FakeClient;
    }

    /**
     * Answers native calls with a FakeClient, which records them, and makes
     * the application behave as in Nativella:
     *
     *     $native = Nativella::fake();
     *     $native->respond('dialog.confirm', ['confirmed' => true]);
     *     // …
     *     $this->assertCount(1, $native->calls('notification.send'));
     */
    public static function fake(): FakeClient
    {
        $fake = new FakeClient();
        self::application()->instance(Client::class, $fake);

        return $fake;
    }

    protected static function getFacadeAccessor(): string
    {
        return \Nativella\Nativella::class;
    }

    private static function application(): Application
    {
        // A test case of PHPUnit itself, without Laravel's.
        return static::getFacadeApplication() ?? throw new \LogicException('Nativella::isNative() and Nativella::fake() need a Laravel application: use the TestCase of Laravel or Testbench.');
    }
}
