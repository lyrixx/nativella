<?php

namespace App\Tests\Controller;

use App\Tests\NotesTestCase;
use Nativella\Exception\NativeCallException;

final class NoteControllerTest extends NotesTestCase
{
    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);

        parent::tearDown();
    }

    public function testTheHomePageInvitesToWriteTheFirstNote(): void
    {
        $this->client->request('GET', '/');

        $this->assertResponseIsSuccessful();
        $this->assertSelectorTextContains('h1', 'No notes yet');
    }

    public function testCreateANote(): void
    {
        $this->client->request('GET', '/notes/new');
        $this->client->submitForm('Save', ['note[title]' => 'Ideas', 'note[body]' => 'Ship **Nativella**.']);

        $this->assertResponseRedirects();
        $this->client->followRedirect();
        $this->assertSelectorTextContains('h1', 'Ideas');
        $this->assertSelectorTextContains('.markdown strong', 'Nativella');
    }

    public function testATitleIsRequired(): void
    {
        $this->client->request('GET', '/notes/new');
        $this->client->submitForm('Save', ['note[title]' => '', 'note[body]' => 'Body']);

        $this->assertResponseStatusCodeSame(422);
        $this->assertCount(0, $this->notes()->findAll());
    }

    public function testEditANote(): void
    {
        $note = $this->createNote();

        $this->client->request('GET', "/notes/{$note->getId()}/edit");
        $this->client->submitForm('Save', ['note[title]' => 'Shopping list']);

        $this->assertResponseRedirects("/notes/{$note->getId()}");
        $this->assertSame('Shopping list', $this->notes()->find($note->getId())?->getTitle());
    }

    public function testSearchTheNotes(): void
    {
        $this->createNote('Groceries', 'milk');
        $this->createNote('Holidays', 'Brittany');

        $this->client->request('GET', '/notes', ['q' => 'BRIT']);

        $this->assertSelectorTextContains('h1', '1 note matches');
        $this->assertSelectorCount(1, '.note-link');
        $this->assertSelectorTextContains('.note-link', 'Holidays');
    }

    public function testCopyAsRichText(): void
    {
        $note = $this->createNote('Title', 'Some *emphasis*');

        $this->client->request('POST', "/notes/{$note->getId()}/copy", ['format' => 'html']);

        $this->assertResponseRedirects();
        $this->assertSame([[
            'method' => 'clipboard.writeHtml',
            'payload' => ['html' => "<p>Some <em>emphasis</em></p>\n", 'altText' => 'Some *emphasis*'],
        ]], $this->fakeClient()->calls());
    }

    public function testAPhoneAddsItsLocation(): void
    {
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Picnic', 'Bring a blanket.');
        $this->fakeClient()->respond('geolocation.currentPosition', ['position' => [
            'latitude' => 45.767299,
            'longitude' => 4.834329,
            'accuracy' => 20,
            'altitude' => null,
            'altitudeAccuracy' => null,
            'speed' => null,
            'heading' => null,
            'timestamp' => 1_760_000_000_000,
        ]]);

        $this->client->request('POST', "/notes/{$note->getId()}/location");

        $this->assertResponseRedirects("/notes/{$note->getId()}");
        $this->assertSame(
            "Bring a blanket.\n\n📍 [45.76730, 4.83433](https://www.openstreetmap.org/?mlat=45.76730&mlon=4.83433#map=17/45.76730/4.83433)",
            $this->notes()->find($note->getId())?->getBody(),
        );
        $this->assertSame('high', $this->fakeClient()->calls('geolocation.currentPosition')[0]['payload']['accuracy']);
    }

    public function testAPhoneTheUserDoesNotLetLocateChangesNothing(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Picnic', 'Bring a blanket.');

        $this->client->request('POST', "/notes/{$note->getId()}/location");
        $this->client->followRedirect();

        $this->assertSelectorTextContains('.flash', 'Notes may not know where you are');
        $this->assertSame('Bring a blanket.', $this->notes()->find($note->getId())?->getBody());
    }

    public function testALocationTurnedOffIsReported(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Picnic', 'Bring a blanket.');
        $this->fakeClient()->respond('geolocation.currentPosition', new NativeCallException('geolocation.currentPosition', new \RuntimeException('Location services are disabled.')));

        $this->client->request('POST', "/notes/{$note->getId()}/location");
        $this->client->followRedirect();

        $this->assertSelectorTextContains('.flash', 'Notes cannot find where you are: Location services are disabled.');
    }

    public function testComputersHaveNoLocation(): void
    {
        $note = $this->createNote();

        $this->client->request('POST', "/notes/{$note->getId()}/location");

        $this->assertResponseStatusCodeSame(404);
        $this->assertSame([], $this->nativeMethods());
    }

    public function testAPhoneAddsAScannedCode(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Shopping', 'Milk.');
        $this->fakeClient()->respond('barcodeScanner.scan', ['barcode' => ['content' => '3017620422003', 'format' => 'EAN_13']]);

        $this->client->request('POST', "/notes/{$note->getId()}/scan");

        $this->assertResponseRedirects("/notes/{$note->getId()}");
        $this->assertSame("Milk.\n\n📷 `3017620422003`", $this->notes()->find($note->getId())?->getBody());
    }

    public function testAScannedWebAddressBecomesALink(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Shopping', '');
        $this->fakeClient()->respond('barcodeScanner.scan', ['barcode' => ['content' => 'https://example.com/a?b=c', 'format' => 'QR_CODE']]);

        $this->client->request('POST', "/notes/{$note->getId()}/scan");

        $this->assertSame('📷 <https://example.com/a?b=c>', $this->notes()->find($note->getId())?->getBody());
    }

    public function testACancelledScanChangesNothing(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Shopping', 'Milk.');

        $this->client->request('POST', "/notes/{$note->getId()}/scan");
        $this->client->followRedirect();

        $this->assertSelectorTextContains('.flash', 'No code was added to the note.');
        $this->assertSame('Milk.', $this->notes()->find($note->getId())?->getBody());
    }

    public function testAFailingCameraIsReported(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $note = $this->createNote('Shopping', 'Milk.');
        $this->fakeClient()->respond('barcodeScanner.scan', new NativeCallException('barcodeScanner.scan', new \RuntimeException('The phone read no barcode within 120 seconds.')));

        $this->client->request('POST', "/notes/{$note->getId()}/scan");
        $this->client->followRedirect();

        $this->assertSelectorTextContains('.flash', 'Notes cannot read a code: The phone read no barcode within 120 seconds.');
    }

    public function testComputersHaveNoBarcodeScanner(): void
    {
        $note = $this->createNote();

        $this->client->request('POST', "/notes/{$note->getId()}/scan");

        $this->assertResponseStatusCodeSame(404);
        $this->assertSame([], $this->nativeMethods());
    }

    public function testExportWritesTheFileTheUserPicked(): void
    {
        $note = $this->createNote();
        $path = $this->client->getKernel()->getCacheDir().'/export/groceries.md';
        $this->fakeClient()->respond('dialog.saveFile', ['path' => $path]);

        $this->client->request('POST', "/notes/{$note->getId()}/export");

        $this->assertResponseRedirects();
        $this->assertStringEqualsFile($path, "# Groceries\n\n- milk\n- bread\n");
        $this->assertSame('groceries.md', $this->fakeClient()->calls('dialog.saveFile')[0]['payload']['fileName']);
        $this->assertSame(['dialog.saveFile', 'notification.send'], $this->nativeMethods());
    }

    public function testACancelledExportWritesNothing(): void
    {
        $note = $this->createNote();

        $this->client->request('POST', "/notes/{$note->getId()}/export");

        $this->assertResponseRedirects();
        $this->assertSame(['dialog.saveFile'], $this->nativeMethods());
    }

    public function testTheContextMenuIsNative(): void
    {
        $note = $this->createNote();

        $this->client->request('POST', "/notes/{$note->getId()}/context-menu");

        $this->assertResponseStatusCodeSame(204);
        $items = $this->fakeClient()->calls('menu.popup')[0]['payload']['items'];
        $this->assertContains("note.delete:{$note->getId()}", array_column($items, 'id'));
    }
}
