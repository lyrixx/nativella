<?php

namespace Nativella\Tests;

use Nativella\Client\Call;
use Nativella\Client\FakeClient;
use Nativella\Event\EventFactory;
use Nativella\Event\GlobalShortcutPressed;
use Nativella\Exception\ErrorKind;
use Nativella\Exception\NativeCallException;
use Nativella\GlobalShortcut;
use PHPUnit\Framework\TestCase;

final class GlobalShortcutTest extends TestCase
{
    public function testRegisterAndUnregister(): void
    {
        $client = new FakeClient();
        $client->respond('globalShortcut.isRegistered', ['registered' => true]);
        $shortcut = new GlobalShortcut($client);

        $shortcut->register('CmdOrCtrl+Shift+N');
        $this->assertTrue($shortcut->isRegistered('CmdOrCtrl+Shift+N'));
        $shortcut->unregister('CmdOrCtrl+Shift+N');
        $shortcut->unregisterAll();
        $this->assertFalse($shortcut->isRegistered('CmdOrCtrl+Shift+N'));

        $this->assertEquals([
            new Call('globalShortcut.register', ['accelerator' => 'CmdOrCtrl+Shift+N']),
            new Call('globalShortcut.isRegistered', ['accelerator' => 'CmdOrCtrl+Shift+N']),
            new Call('globalShortcut.unregister', ['accelerator' => 'CmdOrCtrl+Shift+N']),
            new Call('globalShortcut.unregisterAll', []),
            new Call('globalShortcut.isRegistered', ['accelerator' => 'CmdOrCtrl+Shift+N']),
        ], $client->calls());
    }

    public function testAShortcutAnotherApplicationHolds(): void
    {
        $client = new FakeClient();
        $client->fail('globalShortcut.register', ErrorKind::Failed, 'Already taken.');

        $this->expectException(NativeCallException::class);

        new GlobalShortcut($client)->register('CmdOrCtrl+Shift+N');
    }

    public function testAPress(): void
    {
        $this->assertEquals(new GlobalShortcutPressed('CmdOrCtrl+Shift+N'), EventFactory::create('globalShortcut.pressed', ['accelerator' => 'CmdOrCtrl+Shift+N']));
    }
}
