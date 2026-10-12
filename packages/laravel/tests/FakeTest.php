<?php

namespace Nativella\Laravel\Tests;

use Nativella\Client;
use Nativella\Client\FakeClient;
use Nativella\Dialog;
use Nativella\Exception\RuntimeUnavailableException;
use Nativella\Laravel\Facades\Dialog as DialogFacade;
use Nativella\Laravel\Facades\Nativella;
use Nativella\Laravel\Facades\Notification;
use Nativella\Nativella as NativellaApis;

final class FakeTest extends TestCase
{
    public function testOutsideNativellaNativeCallsFail(): void
    {
        $this->assertFalse(Nativella::isNative());

        $this->expectException(RuntimeUnavailableException::class);

        Notification::send('Saved');
    }

    public function testTheFakeRecordsTheNativeCalls(): void
    {
        $native = Nativella::fake();

        Notification::send('Saved', 'Your note is safe.');

        $this->assertSame($native, app(Client::class));
        $this->assertSame([['method' => 'notification.send', 'payload' => ['title' => 'Saved', 'body' => 'Your note is safe.', 'reference' => null]]], $native->calls());
    }

    public function testTheFakeAnswersAsTheTestSays(): void
    {
        $native = Nativella::fake();
        $native->respond('dialog.confirm', ['confirmed' => true]);

        $this->assertTrue(DialogFacade::confirm('Delete the note?'));
        $this->assertFalse(DialogFacade::confirm('Delete the note?'));
    }

    public function testTheFakeReplacesTheClientOfApisAlreadyResolved(): void
    {
        $dialog = app(Dialog::class);
        DialogFacade::getFacadeRoot();

        $native = Nativella::fake();
        DialogFacade::message('Hello');
        app(Dialog::class)->message('Hello');
        app(NativellaApis::class)->dialog->message('Hello');

        $this->assertNotSame($dialog, app(Dialog::class));
        $this->assertCount(3, $native->calls('dialog.message'));
    }

    public function testTheApplicationBehavesAsInNativella(): void
    {
        Nativella::fake();

        $this->assertTrue(Nativella::isNative());
        $this->assertInstanceOf(FakeClient::class, app(Client::class));
    }
}
