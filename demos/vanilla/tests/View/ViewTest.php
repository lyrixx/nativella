<?php

namespace App\Tests\View;

use App\Snippet\SnippetInput;
use App\Snippet\SnippetSummary;
use App\View\Flash;
use App\View\View;
use PHPUnit\Framework\TestCase;

final class ViewTest extends TestCase
{
    private View $view;

    protected function setUp(): void
    {
        $this->view = new View('/', new Flash(), \dirname(__DIR__, 2).'/templates');
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);
    }

    public function testTheListEscapesWhatTheUserTyped(): void
    {
        $html = $this->view->render('snippets/index', [
            'hasMore' => false,
            'isMobile' => false,
            'isNative' => false,
            'query' => '"><script>',
            'snippets' => [new SnippetSummary(7, '<b>Title</b> & "quotes"', '</pre><script>alert(1)</script>', false, new \DateTimeImmutable('2026-10-11 09:30:00', new \DateTimeZone('UTC')))],
        ]);

        $this->assertStringNotContainsString('<script>', $html);
        $this->assertStringContainsString('<h2>&lt;b&gt;Title&lt;/b&gt; &amp; &quot;quotes&quot;</h2>', $html);
        $this->assertStringContainsString('<pre>&lt;/pre&gt;&lt;script&gt;alert(1)&lt;/script&gt;</pre>', $html);
        $this->assertStringContainsString('value="&quot;&gt;&lt;script&gt;"', $html);
        // Outside Nativella, the browser confirms; the copy button needs the native clipboard.
        $this->assertStringContainsString('data-confirm="Delete “&lt;b&gt;Title&lt;/b&gt; &amp; &quot;quotes&quot;”? This cannot be undone."', $html);
        $this->assertStringNotContainsString('/snippets/7/copy', $html);
        $this->assertStringContainsString('<time datetime="2026-10-11T09:30:00+00:00">', $html);
    }

    public function testInNativellaTheListOffersToCopyAndImport(): void
    {
        $html = $this->view->render('snippets/index', [
            'hasMore' => false,
            'isMobile' => false,
            'isNative' => true,
            'query' => '',
            'snippets' => [new SnippetSummary(7, 'Title', 'Content', true, new \DateTimeImmutable())],
        ]);

        $this->assertStringContainsString('action="/snippets/7/copy" data-copy', $html);
        $this->assertStringContainsString('action="/snippets/import"', $html);
        $this->assertStringNotContainsString('data-confirm', $html);
        $this->assertStringContainsString('<pre>Content…</pre>', $html);
    }

    public function testOnlyDesktopsImportFiles(): void
    {
        $parameters = ['hasMore' => false, 'isNative' => true, 'query' => '', 'snippets' => []];

        $desktop = $this->view->render('snippets/index', ['isMobile' => false] + $parameters);
        $this->assertStringContainsString('Drop text files on the window', $desktop);
        $this->assertStringContainsString('action="/snippets/import"', $desktop);

        $mobile = $this->view->render('snippets/index', ['isMobile' => true] + $parameters);
        $this->assertStringNotContainsString('Drop text files', $mobile);
        $this->assertStringNotContainsString('/snippets/import', $mobile);
    }

    public function testOnMobileThePageKnowsItGoesUnderTheStatusBar(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        $html = $this->view->page('Not found', 'not-found', ['message' => 'Gone.'])->body;

        $this->assertStringContainsString(' data-mobile>', $html);
        $this->assertStringContainsString('viewport-fit=cover', $html);
    }

    public function testOnTheDesktopThePageIsNotMobile(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'linux';

        $this->assertStringNotContainsString('data-mobile', $this->view->page('Not found', 'not-found', ['message' => 'Gone.'])->body);
    }

    public function testTheFormKeepsALeadingLineBreakOfTheContent(): void
    {
        $html = $this->view->render('snippets/new', [
            'errors' => ['content' => 'The content cannot be empty.'],
            'input' => new SnippetInput('<Title>', "\n  indented"),
        ]);

        $this->assertStringContainsString('value="&lt;Title&gt;"', $html);
        $this->assertStringContainsString("aria-invalid=\"true\" aria-describedby=\"content-error\">\n\n  indented</textarea>", $html);
        $this->assertStringContainsString('<p class="error" id="content-error">The content cannot be empty.</p>', $html);
    }
}
