<?php

namespace Nativella\Laravel\Tests;

use Illuminate\Encryption\Encrypter;
use Illuminate\Foundation\Application;
use Orchestra\Testbench\Attributes\DefineEnvironment;

final class InstallationSecretTest extends TestCase
{
    private const string SECRET = '00112233445566778899aabbccddeeff0f1e2d3c4b5a69788796a5b4c3d2e1f0';
    private const string KEY = 'base64:ABEiM0RVZneImaq7zN3u/w8eLTxLWml4h5altMPS4fA=';

    protected function setUp(): void
    {
        // What the runtime sets for every request.
        $_SERVER['NATIVELLA_SECRET'] = self::SECRET;

        parent::setUp();
    }

    protected function tearDown(): void
    {
        parent::tearDown();

        unset($_SERVER['NATIVELLA_SECRET']);
    }

    public function testTheApplicationEncryptsWithTheSecretOfTheInstallation(): void
    {
        $this->assertSame(self::KEY, config('app.key'));

        $encrypted = encrypt('Groceries');
        $this->assertSame('Groceries', new Encrypter(pack('H*', self::SECRET), 'AES-256-CBC')->decrypt($encrypted));
    }

    #[DefineEnvironment('shipTheKeyOfTheDeveloper')]
    public function testTheKeyOfTheShippedEnvFileIsIgnored(): void
    {
        $this->assertSame(self::KEY, config('app.key'));
    }

    #[DefineEnvironment('shipPreviousKeys')]
    public function testThePreviousKeysOfTheShippedEnvFileAreForgotten(): void
    {
        $this->assertSame([], config('app.previous_keys'));
    }

    #[DefineEnvironment('useAes128')]
    public function testAnAes128CipherGetsA16BytesKey(): void
    {
        $this->assertSame('base64:'.base64_encode(substr(pack('H*', self::SECRET), 0, 16)), config('app.key'));
        $this->assertSame('Groceries', decrypt(encrypt('Groceries')));
    }

    protected function shipTheKeyOfTheDeveloper(Application $app): void
    {
        $app['config']->set('app.key', 'base64:'.base64_encode(random_bytes(32)));
    }

    protected function shipPreviousKeys(Application $app): void
    {
        $app['config']->set('app.previous_keys', ['base64:'.base64_encode(random_bytes(32))]);
    }

    protected function useAes128(Application $app): void
    {
        $app['config']->set('app.cipher', 'AES-128-CBC');
    }
}
