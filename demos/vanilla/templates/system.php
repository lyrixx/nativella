<?php
/**
 * @var App\View\View                $this
 * @var Nativella\App\AppInfo|null   $app
 * @var string                       $databasePath
 * @var string|null                  $dataDir
 * @var bool                         $isNative
 * @var string|null                  $runtimeVersion
 * @var list<Nativella\Screen\Display> $screens
 * @var string                       $sqliteVersion
 * @var Nativella\System\SystemInfo|null $system
 * @var string|null                  $window
 */
$size = static fn (Nativella\Screen\Rectangle $rectangle): string => \sprintf('%g × %g at (%g, %g)', $rectangle->width, $rectangle->height, $rectangle->x, $rectangle->y);
$unknown = '<span class="muted">Unknown</span>';
?>
<h1>System</h1>

<?php if (null !== $app && null !== $system): ?>
    <div class="cards">
        <section class="card">
            <h2>Application</h2>
            <dl class="facts">
                <dt>Name</dt><dd><?= $this->e($app->name) ?></dd>
                <dt>Version</dt><dd><?= $this->e($app->version) ?></dd>
                <dt>Identifier</dt><dd><code><?= $this->e($app->identifier) ?></code></dd>
                <dt>Nativella</dt><dd><?= $this->e($app->nativellaVersion) ?></dd>
                <dt>Window</dt><dd><?= null === $window ? $unknown : '<code>'.$this->e($window).'</code>' ?></dd>
            </dl>
        </section>

        <section class="card">
            <h2>Operating system</h2>
            <dl class="facts">
                <dt>System</dt><dd><?= $this->e($system->os) ?> <?= $this->e($system->version) ?> (<?= $this->e($system->family) ?>)</dd>
                <dt>Architecture</dt><dd><?= $this->e($system->arch) ?></dd>
                <dt>Host name</dt><dd><?= null === $system->hostname ? $unknown : $this->e($system->hostname) ?></dd>
                <dt>Locale</dt><dd><?= null === $system->locale ? $unknown : $this->e($system->locale) ?></dd>
            </dl>
        </section>
    </div>

    <section class="card">
        <h2>Displays</h2>
        <?php if ([] === $screens): ?>
            <p class="muted">The system reports no display.</p>
        <?php else: ?>
            <table>
                <thead>
                    <tr><th>Name</th><th>Size and position</th><th>Work area</th><th>Scale</th></tr>
                </thead>
                <tbody>
                    <?php foreach ($screens as $screen): ?>
                        <tr>
                            <td>
                                <?= null === $screen->name ? $unknown : $this->e($screen->name) ?>
                                <?php if ($screen->primary): ?><span class="badge">Primary</span><?php endif ?>
                            </td>
                            <td><?= $size($screen->bounds) ?></td>
                            <td><?= $size($screen->workArea) ?></td>
                            <td><?= $screen->scaleFactor ?>×</td>
                        </tr>
                    <?php endforeach ?>
                </tbody>
            </table>
            <p class="hint">In logical pixels, as CSS and window sizes are.</p>
        <?php endif ?>
    </section>
<?php else: ?>
    <p class="card muted">The application, the operating system and the displays are only known when the app runs in Nativella.</p>
<?php endif ?>

<section class="card">
    <h2>Storage and PHP</h2>
    <dl class="facts">
        <dt>Data directory</dt><dd><?= null === $dataDir ? '<span class="muted">None outside Nativella</span>' : '<code>'.$this->e($dataDir).'</code>' ?></dd>
        <dt>Database</dt><dd><code><?= $this->e($databasePath) ?></code></dd>
        <dt>SQLite</dt><dd><?= $this->e($sqliteVersion) ?></dd>
        <dt>PHP</dt><dd><?= \PHP_VERSION ?>, <code><?= \PHP_SAPI ?></code> SAPI</dd>
        <dt>Runtime</dt><dd><?= null === $runtimeVersion ? '<span class="muted">Not Nativella</span>' : 'Nativella '.$this->e($runtimeVersion) ?></dd>
    </dl>
</section>
