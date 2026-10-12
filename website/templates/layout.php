<?php
/**
 * @var Nativella\Website\View $this
 * @var string                 $content
 * @var string                 $title
 * @var string                 $description
 * @var string                 $bodyClass
 */
?>
<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title><?= $this->e($title) ?></title>
    <meta name="description" content="<?= $this->e($description) ?>">
    <link rel="canonical" href="<?= $this->e($this->absolute()) ?>">

    <meta property="og:type" content="website">
    <meta property="og:site_name" content="Nativella">
    <meta property="og:title" content="<?= $this->e($title) ?>">
    <meta property="og:description" content="<?= $this->e($description) ?>">
    <meta property="og:url" content="<?= $this->e($this->absolute()) ?>">
    <meta property="og:image" content="<?= $this->e($this->absolute('assets/og.png')) ?>">
    <meta property="og:image:width" content="1200">
    <meta property="og:image:height" content="630">
    <meta property="og:image:alt" content="Nativella: your PHP app, in a window of its own.">
    <meta name="twitter:card" content="summary_large_image">

    <meta name="color-scheme" content="light dark">
    <meta name="theme-color" content="#f6f0e5" media="(prefers-color-scheme: light)">
    <meta name="theme-color" content="#15120e" media="(prefers-color-scheme: dark)">
    <link rel="icon" href="<?= $this->asset('favicon.svg') ?>" type="image/svg+xml">
    <link rel="apple-touch-icon" href="<?= $this->asset('apple-touch-icon.png') ?>">
    <link rel="preload" href="<?= $this->url('assets/fonts/fraunces.woff2') ?>" as="font" type="font/woff2" crossorigin>
    <link rel="stylesheet" href="<?= $this->asset('site.css') ?>">
    <script>document.documentElement.classList.add('js');</script>
    <script src="<?= $this->asset('site.js') ?>" defer></script>
</head>
<body class="<?= $this->e($bodyClass) ?>">
    <a class="skip-link" href="#content">Skip to content</a>
    <?= $this->partial('partials/header') ?>
    <?= $content ?>
    <?= $this->partial('partials/footer') ?>
</body>
</html>
