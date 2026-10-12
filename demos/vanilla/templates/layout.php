<?php
/**
 * @var App\View\View $this
 * @var string        $content     the page, already rendered
 * @var string        $currentPath
 * @var list<string>  $flashes
 * @var bool          $isMobile
 * @var bool          $isNative
 * @var string        $title
 */
$sections = ['/' => 'Snippets', '/system' => 'System'];
$currentSection = str_starts_with($currentPath, '/system') ? '/system' : '/';
?>
<!DOCTYPE html>
<html lang="en" data-native="<?= $isNative ? 'true' : 'false' ?>"<?= $isMobile ? ' data-mobile' : '' ?>>
<head>
    <meta charset="UTF-8">
    <?php /* viewport-fit=cover: on phones, the page goes under the status bar, see app.css. */ ?>
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="color-scheme" content="light dark">
    <title><?= $this->e($title) ?></title>
    <?php /* Or the webview asks PHP for /favicon.ico on every page. */ ?>
    <link rel="icon" href="/icon.svg">
    <link rel="stylesheet" href="/app.css">
    <script src="/app.js" defer></script>
</head>
<body>
    <header class="toolbar">
        <nav class="segmented" aria-label="Sections">
            <?php foreach ($sections as $path => $label): ?>
                <a href="<?= $path ?>"<?= $path === $currentSection ? ' aria-current="page"' : '' ?>><?= $label ?></a>
            <?php endforeach ?>
        </nav>
    </header>

    <?php if (!$isNative): ?>
        <p class="banner">Running in a browser: copying, importing, notifications and native dialogs need Nativella.</p>
    <?php endif ?>

    <main>
        <?php foreach ($flashes as $flash): ?>
            <p class="flash" role="status"><?= nl2br($this->e($flash)) ?></p>
        <?php endforeach ?>

        <?= $content ?>
    </main>

    <div id="toast" class="toast" role="status" aria-live="polite" hidden></div>
</body>
</html>
