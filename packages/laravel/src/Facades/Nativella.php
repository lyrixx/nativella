<?php

namespace Nativella\Laravel\Facades;

use Illuminate\Contracts\Foundation\Application;
use Illuminate\Support\Facades\Facade;
use Nativella\Client;
use Nativella\Client\FakeClient;

/**
 * Its root holds every API, as Nativella\Nativella does.
 *
 * @see \Nativella\Nativella
 */
final class Nativella extends Facade
{
    /**
     * Native calls work when the application runs in Nativella, or when the
     * tests faked it. Offer native features only then, with @native in Blade.
     */
    public static function isNative(): bool
    {
        return self::application()->make(Client::class)->isNative();
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
        // PHPUnit's own test case, without Laravel's.
        return static::getFacadeApplication() ?? throw new \LogicException('Nativella::isNative() and Nativella::fake() need a Laravel application: use the TestCase of Laravel or Testbench.');
    }
}
