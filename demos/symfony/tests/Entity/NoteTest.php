<?php

namespace App\Tests\Entity;

use App\Entity\Note;
use PHPUnit\Framework\TestCase;

final class NoteTest extends TestCase
{
    public function testAMarkdownFileGivesItsHeadingAsTitle(): void
    {
        $note = Note::fromFile('whatever.md', "# Pancakes\r\n\r\nFlour, eggs.\r\n");

        $this->assertSame('Pancakes', $note->getTitle());
        $this->assertSame('Flour, eggs.', $note->getBody());
    }

    public function testAFileWithoutHeadingIsNamedAfterTheFile(): void
    {
        $note = Note::fromFile('shopping list.txt', 'milk');

        $this->assertSame('shopping list', $note->getTitle());
        $this->assertSame('milk', $note->getBody());
    }

    public function testExportAndImportRoundTrip(): void
    {
        $note = new Note('Title', "Line 1\n\nLine 2");

        $this->assertEquals($note->getBody(), Note::fromFile('x.md', $note->toMarkdown())->getBody());
    }

    public function testACodeWithBackticksStaysInItsSpan(): void
    {
        $note = new Note('Title', 'Body');

        $note->addCode('a`b');
        $note->addCode('`b`');

        $this->assertSame("Body\n\n📷 ``a`b``\n\n📷 `` `b` ``", $note->getBody());
    }
}
