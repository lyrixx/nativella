<?php

namespace App\Tests\Controller;

use App\Tests\NotesTestCase;
use PHPUnit\Framework\Attributes\DataProvider;

/**
 * The application in a browser, outside Nativella.
 */
final class BrowserTest extends NotesTestCase
{
    protected function setUp(): void
    {
        parent::setUp();

        $this->leaveNativella();
    }

    public function testANoteHidesTheNativeFeatures(): void
    {
        $note = $this->createNote();

        $this->client->request('GET', "/notes/{$note->getId()}");

        $this->assertResponseIsSuccessful();
        $this->assertSelectorTextContains('.flash.warning', 'Running in a browser');
        $this->assertSelectorTextNotContains('.actions', 'Copy');
        $this->assertSelectorTextNotContains('.sidebar-actions', 'Import');
        $this->assertSelectorExists('form[data-confirm]');
    }

    public function testTheEmptyPageNamesNoShortcut(): void
    {
        $this->client->request('GET', '/');

        $this->assertSelectorTextContains('h1', 'No notes yet');
        $this->assertSelectorNotExists('.shortcuts');
    }

    public function testDeletingANoteAsksNothingMore(): void
    {
        $note = $this->createNote();
        $id = $note->getId();

        // The page asked with confirm() already.
        $this->post("/notes/{$id}/delete");

        $this->assertResponseRedirects('/');
        $this->assertNull($this->notes()->find($id));
    }

    public function testTheSettingsAreOnlyInNativella(): void
    {
        $this->client->request('GET', '/settings');

        $this->assertResponseIsSuccessful();
        $this->assertSelectorTextContains('.settings', 'open the application in Nativella');
        $this->assertSelectorNotExists('.settings form');
    }

    #[DataProvider('provideNativeRoutes')]
    public function testTheNativeRoutesDoNotExist(string $uri): void
    {
        $note = $this->createNote();

        $this->post(str_replace('{id}', (string) $note->getId(), $uri));

        $this->assertResponseStatusCodeSame(404);
    }

    /**
     * @return iterable<array{string}>
     */
    public static function provideNativeRoutes(): iterable
    {
        yield ['/notes/{id}/copy'];
        yield ['/notes/{id}/export'];
        yield ['/notes/{id}/context-menu'];
        yield ['/notes/{id}/share'];
        yield ['/notes/import'];
        yield ['/settings'];
        yield ['/settings/open-data-dir'];
    }
}
