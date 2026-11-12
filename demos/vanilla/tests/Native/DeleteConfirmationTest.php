<?php

namespace App\Tests\Native;

use App\Native\DeleteConfirmation;
use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Exception\ErrorKind;
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

    public function testPhonesAskWhoHoldsThem(): void
    {
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $this->client->respond('biometrics.authenticate', ['authenticated' => true]);

        $this->assertTrue($this->confirmation->confirm('Delete “SSH”?'));
        // Without asking whether a fingerprint is enrolled: the phone falls back on its PIN.
        $this->assertEquals(
            [new Call('biometrics.authenticate', ['reason' => 'Delete “SSH”?', 'title' => 'Delete the snippet'])],
            $this->client->calls(),
        );
    }

    public function testSomeoneElseDoesNotConfirm(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $this->client->respond('biometrics.authenticate', ['authenticated' => false]);

        $this->assertFalse($this->confirmation->confirm('Delete “SSH”?'));
        $this->assertSame([], $this->client->calls('dialog.confirm'));
    }

    public function testAPhoneWithoutALockAsksWithADialog(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $this->client->fail('biometrics.authenticate', ErrorKind::Failed, 'The phone has no lock.');
        $this->client->respond('dialog.confirm', ['confirmed' => true]);

        $this->expectErrorLog();
        $this->assertTrue($this->confirmation->confirm('Delete “SSH”?'));
        $this->assertSame(['biometrics.authenticate', 'dialog.confirm'], array_column($this->client->calls(), 'method'));
    }
}
