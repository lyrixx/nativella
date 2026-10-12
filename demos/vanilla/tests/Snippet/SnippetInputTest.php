<?php

namespace App\Tests\Snippet;

use App\Snippet\SnippetInput;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class SnippetInputTest extends TestCase
{
    public function testFromFormTrimsTheTitleAndNormalizesLineBreaks(): void
    {
        $input = SnippetInput::fromForm("  Deploy \n", "  make deploy\r\nmake clean\r\n");

        $this->assertSame('Deploy', $input->title);
        // The content keeps its indentation: it may be code.
        $this->assertSame("  make deploy\nmake clean\n", $input->content);
    }

    public function testAValidInputHasNoError(): void
    {
        $this->assertSame([], new SnippetInput('Café', "Un café, s'il vous plaît.")->validate());
    }

    /**
     * @param array<string, string> $errors
     */
    #[DataProvider('provideInvalidInputs')]
    public function testValidate(string $title, string $content, array $errors): void
    {
        $this->assertSame($errors, new SnippetInput($title, $content)->validate());
    }

    /**
     * @return iterable<string, array{string, string, array<string, string>}>
     */
    public static function provideInvalidInputs(): iterable
    {
        yield 'empty' => ['', '', ['title' => 'The title cannot be empty.', 'content' => 'The content cannot be empty.']];
        yield 'blank' => [' ', "\n\t", ['title' => 'The title cannot be empty.', 'content' => 'The content cannot be empty.']];
        yield 'title too long' => [str_repeat('é', 256), 'x', ['title' => 'The title cannot be longer than 255 characters.']];
        yield 'content too large' => ['Big', str_repeat('x', SnippetInput::MAX_CONTENT_SIZE + 1), ['content' => 'The content cannot be larger than 1 MiB.']];
        yield 'invalid UTF-8' => ["\xFF", "\xC3\x28", ['title' => 'The title must be UTF-8 text.', 'content' => 'The content must be UTF-8 text.']];
        yield 'binary' => ['image.png', "\x89PNG\r\n\x1A\n\0\0\0\rIHDR", ['content' => 'The content must be UTF-8 text.']];
    }

    public function testTheLimitsAreInclusive(): void
    {
        $this->assertSame([], new SnippetInput(str_repeat('é', 255), str_repeat('x', SnippetInput::MAX_CONTENT_SIZE))->validate());
    }
}
