<?php
/**
 * @var App\View\View                       $this
 * @var bool                                $hasMore
 * @var bool                                $isMobile
 * @var bool                                $isNative
 * @var string                              $query
 * @var list<App\Snippet\SnippetSummary>    $snippets
 */
use App\Controller\SnippetController;

?>
<div class="page-header">
    <form class="search" method="get" action="/" role="search">
        <input type="search" id="search" name="q" value="<?= $this->e($query) ?>" placeholder="Search" aria-label="Search the snippets" autocomplete="off" spellcheck="false">
    </form>
    <?php if ($isNative && !$isMobile): ?>
        <form method="post" action="/snippets/import">
            <button type="submit">Import…</button>
        </form>
    <?php endif ?>
    <a class="button primary" href="/snippets/new">New snippet</a>
</div>

<?php /* public/app.js replaces this section after a search or an import. */ ?>
<section id="snippets">
    <?php if ([] === $snippets): ?>
        <div class="empty">
            <?php if ('' !== $query): ?>
                <h1>No results</h1>
                <p>No snippet contains “<?= $this->e($query) ?>”.</p>
            <?php else: ?>
                <h1>No snippets yet</h1>
                <p>
                    Save the text you paste over and over: commands, replies, addresses…
                    <?php if ($isNative && !$isMobile): ?>
                        <br>Drop text files on the window to import them.
                    <?php endif ?>
                </p>
                <p><a class="button primary" href="/snippets/new">New snippet</a></p>
            <?php endif ?>
        </div>
    <?php else: ?>
        <ul class="snippets">
            <?php foreach ($snippets as $snippet): ?>
                <li class="snippet">
                    <div class="snippet-header">
                        <h2><?= $this->e($snippet->title) ?></h2>
                        <time datetime="<?= $snippet->createdAt->format(\DATE_ATOM) ?>"><?= $snippet->createdAt->format('Y-m-d H:i') ?> UTC</time>
                    </div>
                    <pre><?= $this->e($snippet->excerpt) ?><?= $snippet->truncated ? '…' : '' ?></pre>
                    <div class="snippet-actions">
                        <?php if ($isNative): ?>
                            <form method="post" action="/snippets/<?= $snippet->id ?>/copy" data-copy>
                                <button type="submit">Copy</button>
                            </form>
                        <?php endif ?>
                        <?php /* Phones save no download yet. */ ?>
                        <?php if (!$isMobile): ?>
                            <a class="button" href="/snippets/<?= $snippet->id ?>/download">Download</a>
                        <?php endif ?>
                        <?php /* Nativella asks with a native dialog, a browser with confirm(). */ ?>
                        <form method="post" action="/snippets/<?= $snippet->id ?>/delete"<?php if (!$isNative): ?> data-confirm="<?= $this->e(sprintf(SnippetController::DELETE_QUESTION, $snippet->title)) ?>"<?php endif ?>>
                            <input type="hidden" name="q" value="<?= $this->e($query) ?>">
                            <button type="submit" class="danger">Delete</button>
                        </form>
                    </div>
                </li>
            <?php endforeach ?>
        </ul>
        <?php if ($hasMore): ?>
            <p class="hint">Only the <?= SnippetController::LIST_LIMIT ?> most recent snippets are listed: search to find the others.</p>
        <?php endif ?>
    <?php endif ?>
</section>
