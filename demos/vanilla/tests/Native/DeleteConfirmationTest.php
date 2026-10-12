<?php

namespace App\Tests\Native;

use App\Native\DeleteConfirmation;
use Nativella\Client\FakeClient;
use Nativella\Nativella;
use PHPUnit\Framework\TestCase;

final class DeleteConfirmationTest extends TestCase
{
    private FakeClient $client;
    private DeleteConfirmation $confirmation;

    protected function setUp(): void
    {
        $this->client = new FakeClient();
        $this->confirmation = new DeleteConfirmation(new Nativella($this->client));
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);
    }

    public function testADialogAsksOnComputers(): void
    {
        $this->client->respond('dialog.confirm', ['confirmed' => true]);

        $this->assertTrue($this->confirmation->confirm('Delete “SSH”?'));
        $this->assertSame(['dialog.confirm'], array_column($this->client->calls(), 'method'));
    }

    public function testAFingerprintConfirmsOnPhones(): void
    {
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $this->client->respond('biometrics.isAvailable', ['available' => true]);
        $this->client->respond('biometrics.authenticate', ['authenticated' => true]);

        $this->assertTrue($this->confirmation->confirm('Delete “SSH”?'));
        $this->assertSame(['reason' => 'Delete “SSH”?', 'title' => 'Delete the snippet'], $this->client->calls('biometrics.authenticate')[0]['payload']);
    }

    public function testAPhoneWithoutBiometricsAsksWithADialog(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        $this->assertFalse($this->confirmation->confirm('Delete “SSH”?'));
        $this->assertSame(['biometrics.isAvailable', 'dialog.confirm'], array_column($this->client->calls(), 'method'));
    }
}
