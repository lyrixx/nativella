<?php

namespace App\Tests\EventListener;

use App\Native\Desktop;
use App\Tests\NotesTestCase;
use Nativella\Exception\NativeCallException;

final class NativeEventsTest extends NotesTestCase
{
    public function testTheApplicationSetsItselfUpAtBoot(): void
    {
        $this->sendNativeEvent('app.booted');

        $this->assertSame(['settings.get', 'menu.set', 'tray.set', 'globalShortcut.register', 'settings.get', 'window.open'], $this->nativeMethods());
        $this->assertSame(['accelerator' => Desktop::QUICK_NOTE_SHORTCUT], $this->fakeClient()->calls('globalShortcut.register')[0]->payload);
        $this->assertSame('Nativella Notes: 0 notes', $this->fakeClient()->calls('tray.set')[0]->payload['tooltip']);
    }

    public function testATakenShortcutStillOpensTheWindow(): void
    {
        $this->fakeClient()->respond('globalShortcut.register', new NativeCallException('globalShortcut.register', new \RuntimeException('Already taken.')));

        $this->sendNativeEvent('app.booted');

        $this->assertCount(1, $this->fakeClient()->calls('window.open'));
    }

    public function testDeleteFromTheContextMenuAsksFirst(): void
    {
        $note = $this->createNote();
        $id = $note->getId();

        $this->sendNativeEvent('menu.itemClicked', ['id' => "note.delete:$id", 'checked' => null]);
        $this->assertNotNull($this->notes()->find($id), 'The user said no.');

        $this->fakeClient()->respond('dialog.confirm', ['confirmed' => true]);
        $this->sendNativeEvent('menu.itemClicked', ['id' => "note.delete:$id", 'checked' => null]);

        $this->entityManager()->clear();
        $this->assertNull($this->notes()->find($id));
        $this->assertEquals(['event' => 'notes.changed', 'detail' => (object) ['deleted' => $id], 'window' => null], $this->fakeClient()->calls('page.dispatch')[0]->payload);
    }

    public function testAContextMenuOfADeletedNoteDoesNothing(): void
    {
        $this->sendNativeEvent('menu.itemClicked', ['id' => 'note.delete:999999', 'checked' => null]);

        $this->assertSame([], $this->fakeClient()->calls());
    }

    public function testDroppedMarkdownFilesBecomeNotes(): void
    {
        $dir = $this->client->getKernel()->getCacheDir().'/dropped';
        @mkdir($dir, recursive: true);
        file_put_contents("$dir/recipe.md", "# Pancakes\n\nFlour, eggs, milk.");
        file_put_contents("$dir/photo.png", 'not a note');

        $this->sendNativeEvent('window.filesDropped', ['window' => 'main', 'paths' => ["$dir/recipe.md", "$dir/photo.png"]]);

        $this->assertSame(['Pancakes'], array_map(static fn ($note) => $note->getTitle(), $this->notes()->findAll()));
        $this->assertSame(
            ['title' => 'Nativella Notes', 'body' => 'Imported "Pancakes". Skipped photo.png: only Markdown and text files in UTF-8, up to 1 MB, are supported.', 'reference' => null],
            $this->fakeClient()->calls('notification.send')[0]->payload,
        );
    }

    public function testDroppedFilesThatAreNotUtf8TextAreSkipped(): void
    {
        $fixtures = \dirname(__DIR__).'/Fixtures';

        $this->sendNativeEvent('window.filesDropped', ['window' => 'main', 'paths' => ["$fixtures/latin1.txt", "$fixtures/binary.txt"]]);

        $this->assertSame([], $this->notes()->findAll());
        $this->assertSame(
            'No note imported. Skipped latin1.txt, binary.txt: only Markdown and text files in UTF-8, up to 1 MB, are supported.',
            $this->fakeClient()->calls('notification.send')[0]->payload['body'],
        );
        $this->assertSame([], $this->fakeClient()->calls('page.dispatch'));
    }

    public function testAnExportFromTheContextMenuThatCannotBeWrittenIsShown(): void
    {
        $note = $this->createNote();
        $this->fakeClient()->respond('dialog.saveFile', ['path' => __FILE__.'/groceries.md']);

        $this->sendNativeEvent('menu.itemClicked', ['id' => "note.export:{$note->getId()}", 'checked' => null]);

        $this->assertSame('error', $this->fakeClient()->calls('dialog.message')[0]->payload['kind']);
        $this->assertSame([], $this->fakeClient()->calls('notification.send'));
    }

    public function testDeveloperToolsThatDoNotExistAreExplained(): void
    {
        $this->fakeClient()->respond('window.openDevtools', new NativeCallException('window.openDevtools', new \RuntimeException('Developer tools only exist in debug builds.')));

        $this->sendNativeEvent('menu.itemClicked', ['id' => 'view.devtools', 'checked' => null]);

        $this->assertCount(1, $this->fakeClient()->calls('dialog.message'));
    }

    public function testTheAlwaysOnTopCheckboxIsRemembered(): void
    {
        $this->sendNativeEvent('menu.itemClicked', ['id' => 'view.alwaysOnTop', 'checked' => true]);

        $this->assertSame(['key' => Desktop::SETTING_ALWAYS_ON_TOP, 'value' => true], $this->fakeClient()->calls('settings.set')[0]->payload);
        $this->assertSame(['enabled' => true, 'window' => 'main'], $this->fakeClient()->calls('window.setAlwaysOnTop')[0]->payload);
    }

    public function testADeepLinkOpensItsNote(): void
    {
        $note = $this->createNote();

        $this->sendNativeEvent('app.urlsOpened', ['urls' => ["nativella-notes://notes/{$note->getId()}"]]);

        $this->assertSame(['url' => "/notes/{$note->getId()}", 'window' => 'main'], $this->fakeClient()->calls('window.navigate')[0]->payload);
    }

    public function testADeepLinkToNothingIsIgnored(): void
    {
        $this->sendNativeEvent('app.urlsOpened', ['urls' => ['nativella-notes://settings']]);
        $this->sendNativeEvent('app.urlsOpened', ['urls' => ['nativella-notes://notes/999999']]);

        $this->assertSame([], $this->fakeClient()->calls('window.navigate'));
    }

    public function testAPageCannotFakeANativeEvent(): void
    {
        $this->client->request('POST', '/_nativella/event', content: '{"name":"app.booted","payload":{}}');

        $this->assertResponseStatusCodeSame(404);
        $this->assertSame([], $this->fakeClient()->calls());
    }
}
