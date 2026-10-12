<?php
/**
 * @var App\View\View                   $this
 * @var array<'content'|'title', string> $errors
 * @var App\Snippet\SnippetInput        $input
 */
$invalid = static fn (string $field): string => isset($errors[$field]) ? ' aria-invalid="true" aria-describedby="'.$field.'-error"' : '';
?>
<form class="editor" method="post" action="/snippets">
    <h1>New snippet</h1>

    <div class="field">
        <label for="title">Title</label>
        <input id="title" name="title" value="<?= $this->e($input->title) ?>" autocomplete="off" autofocus<?= $invalid('title') ?>>
        <?php if (isset($errors['title'])): ?>
            <p class="error" id="title-error"><?= $this->e($errors['title']) ?></p>
        <?php endif ?>
    </div>

    <div class="field">
        <label for="content">Content</label>
        <?php /* HTML drops a line break right after <textarea>: this one, not the content's. */ ?>
        <textarea id="content" name="content" rows="14" spellcheck="false"<?= $invalid('content') ?>><?= "\n".$this->e($input->content) ?></textarea>
        <?php if (isset($errors['content'])): ?>
            <p class="error" id="content-error"><?= $this->e($errors['content']) ?></p>
        <?php endif ?>
    </div>

    <div class="form-actions">
        <a class="button" href="/">Cancel</a>
        <button type="submit" class="primary">Save</button>
    </div>
</form>
