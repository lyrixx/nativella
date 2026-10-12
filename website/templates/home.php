<?php
/**
 * @var Nativella\Website\View $this
 * @var string                 $architecture the diagram of docs/images/architecture.svg
 */
$this->extend('layout', [
    'title' => 'Nativella: your PHP app, in a window of its own',
    'description' => 'Nativella runs a Symfony, Laravel or plain PHP application as a desktop application: a native window, PHP in the same process, no web server, no open port, no Electron.',
    'bodyClass' => 'page-home',
]);

$apis = [
    ['windows', 'Windows', 'Open, move, resize and hide windows, each known by its label. Print one to PDF.', "\$window->open('settings')"],
    ['menus', 'Menus', 'The application menu and context menus. Clicks come back as events.', '$menu->popup(...$items)'],
    ['tray', 'Tray', 'An icon in the system tray, with a menu. The app lives on without windows.', "\$tray->set('Notes')"],
    ['dialogs', 'Dialogs', 'Messages, questions, file pickers. The answer is a return value.', "\$dialog->confirm('Sure?')"],
    ['notifications', 'Notifications', 'Desktop notifications, from any request. A click comes back as an event.', "\$notification->send('Hi')"],
    ['clipboard', 'Clipboard', 'Read and write text and HTML, from PHP.', '$clipboard->readText()'],
    ['global-shortcuts', 'Global shortcuts', 'Keyboard shortcuts that fire while the app is in the background.', 'GlobalShortcutPressed'],
    ['settings', 'Settings', 'A key-value store for preferences, kept as JSON in the data directory.', "\$settings->get('theme')"],
    ['secrets', 'Secrets', 'Tokens and passwords in the keychain of the system, encrypted by it.', "\$secrets->get('token')"],
    ['timers', 'Timers', 'Wake PHP up for periodic work: a sync, a cleanup, a reminder.', "\$timer->every('sync', 300)"],
    ['child-processes', 'Child processes', 'Start a worker or a tool. Its output and its end come back as events.', 'ChildProcessOutput $event'],
    ['power', 'Power', 'Battery level, power source, and an event when the computer wakes up.', '$power->isOnBattery()'],
    ['shell', 'Shell', 'Open URLs in the browser, files with their application, folders in the file manager.', '$shell->openUrl($url)'],
    ['screens', 'Screens', 'Displays, work areas, scale factors and the cursor.', '$screen->primary()'],
    ['system', 'System', 'The operating system, its locale, and the light or dark theme.', '$system->theme()'],
    ['javascript', 'Pages', 'Push events from PHP to the JavaScript of any window.', "\$page->dispatch('saved')"],
    ['app', 'App', 'Identity, directories, autostart, badge count, quit and restart.', '$app->enableAutostart()'],
    ['lifecycle', 'Native events', 'Menu clicks, shortcuts, dropped files, focus and theme changes, as typed objects.', 'MenuItemClicked $event'],
    ['database', 'Database', 'SQLite in the data directory, migrated when the app starts.', 'NATIVELLA_DATA_DIR'],
    ['configuration', 'Deep links', 'notes://42 in a browser or a mail opens the app, and PHP gets the URL.', 'AppUrlsOpened $event'],
    ['building', 'Standalone build', 'One executable that carries your whole project, and its PHP.', 'nativella build notes'],
    ['mobile', 'Android', 'The same project in an APK, experimentally.', 'mobile/android/build.sh'],
    ['phone-apis', 'Phone APIs', 'Vibrations, the location of the phone, fingerprint and face authentication.', "\$biometrics->authenticate('Unlock')"],
];
?>
<main id="content">
    <section class="hero">
        <div class="wrap hero-grid">
            <div class="hero-text">
                <a class="hero-eyebrow" href="#status"><span class="pulse" aria-hidden="true"></span> <span class="hero-eyebrow-wide">Open source · </span>v<?= $this->e($this->site->version) ?> · desktop, and Android (experimental)</a>
                <h1>Your PHP app, in a window of its&nbsp;own.</h1>
                <p class="hero-lede">Nativella runs Symfony, Laravel or plain PHP as a desktop application, and on Android, experimentally. The window hands every request straight to PHP, inside the same process: no web server, no open port, no Electron, no Node.</p>
                <div class="hero-actions">
                    <a class="button button-primary" href="<?= $this->url('docs/getting-started/') ?>">Get started <span aria-hidden="true">→</span></a>
                    <a class="button button-secondary" href="#how-it-works">How it works</a>
                </div>
                <p class="hero-command"><code><span aria-hidden="true">$ </span>nativella run path/to/your/app</code></p>
            </div>

            <figure class="hero-visual" role="img" aria-label="A Notes window served by PHP, a desktop notification, and the log of the requests the runtime served: no server involved.">
                <div class="mock-window">
                    <div class="mock-titlebar">
                        <span class="mock-icon"></span>
                        <span class="mock-title">Notes</span>
                        <span class="mock-controls"><i></i><i></i><i></i></span>
                    </div>
                    <div class="mock-menubar"><span>File</span><span>Edit</span><span>View</span></div>
                    <div class="mock-body">
                        <ul class="mock-list">
                            <li class="is-active"><b>Groceries</b><span>Eggs, flour, basil</span></li>
                            <li><b>Release notes</b><span>v0.1, the first window</span></li>
                            <li><b>Ideas</b><span>A tray icon for…</span></li>
                        </ul>
                        <div class="mock-note">
                            <p class="mock-note-title">Groceries</p>
                            <p>Eggs, flour, basil. Ask PHP to remember the rest.</p>
                            <span class="mock-line"></span>
                            <span class="mock-line mock-line-short"></span>
                            <span class="mock-line mock-line-shorter"></span>
                        </div>
                    </div>
                </div>
                <div class="mock-toast">
                    <span class="mock-icon"></span>
                    <span><b>Export finished</b> notes.zip is in your Downloads.</span>
                </div>
                <div class="mock-log">
                    <div class="mock-log-bar"><span>stderr</span>$ nativella run notes</div>
<pre><span class="log-tag log-muted">[nativella]</span> Data directory: ~/.local/share/com.acme.notes
<span class="log-tag log-php">[php]</span> POST php://localhost/_nativella/event 204 <b>3.1 ms</b>
<span class="log-tag log-php">[php]</span> GET php://localhost/ 200 <b>4.8 ms</b>
<span class="log-tag log-static">[static]</span> GET php://localhost/app.css 200
<span class="log-tag log-php">[php]</span> POST php://localhost/notes/42 303 <b>5.2 ms</b>
<span class="log-tag log-redirect">[redirect]</span> 303 to /notes/42, replaced by a client-side navigation.</pre>
                </div>
            </figure>
        </div>
    </section>

    <section class="unpacked" aria-labelledby="unpacked-title">
        <div class="wrap unpacked-inner">
            <h2 id="unpacked-title">What you don’t ship</h2>
            <ul class="unpacked-list">
                <li><s>nginx</s></li>
                <li><s>PHP-FPM</s></li>
                <li><s>localhost:8000</s></li>
                <li><s>a copy of Chromium</s></li>
                <li><s>Node.js</s></li>
                <li><s>Electron</s></li>
            </ul>
            <p>What is left: your PHP application, a small Rust shell, and the webview the system already has.</p>
        </div>
    </section>

    <section class="section" aria-labelledby="why-title">
        <div class="wrap">
            <div class="section-head">
                <p class="kicker">Why it is different</p>
                <h2 id="why-title">A desktop app that is still, underneath, a PHP app.</h2>
            </div>
            <div class="pillars">
                <article class="pillar">
                    <span class="pillar-number">01</span>
                    <h3>In-process, not over HTTP</h3>
                    <p>Requests travel from the webview to PHP in memory, through a SAPI written in Rust. Nothing listens on <code>127.0.0.1</code>, so nothing else on the machine can talk to your app.</p>
                </article>
                <article class="pillar">
                    <span class="pillar-number">02</span>
                    <h3>The webview of the system</h3>
                    <p>WebKitGTK on Linux, WKWebView on macOS, WebView2 on Windows. No browser engine in every download, no JavaScript runtime next to it.</p>
                </article>
                <article class="pillar">
                    <span class="pillar-number">03</span>
                    <h3>Your front controller, untouched</h3>
                    <p><code>$_SERVER</code>, <code>$_POST</code>, cookies, sessions, <code>header()</code>: PHP behaves as under PHP-FPM. If it runs behind nginx, it should run here.</p>
                </article>
                <article class="pillar">
                    <span class="pillar-number">04</span>
                    <h3>One file to hand over</h3>
                    <p><code>nativella build</code> packs the project into a single executable, and the same bundle goes into an Android APK. Built against a static PHP, the executable needs no PHP on the machine.</p>
                </article>
            </div>
        </div>
    </section>

    <section class="section section-tinted" aria-labelledby="php-title">
        <div class="wrap split">
            <div class="split-text">
                <p class="kicker">Plain PHP objects</p>
                <h2 id="php-title">Native features, one method call away.</h2>
                <p>Every native API is a small, typed PHP class: named arguments, enums, value objects, and exceptions when the system says no. Inject them in Symfony or Laravel, or get them all from <code>Nativella::create()</code>.</p>
                <p>The other way around, what happens natively reaches PHP as typed events: a menu click, a global shortcut, files dropped on a window. And the page’s JavaScript, as <code>nativella:*</code> DOM events.</p>
                <p><a class="link-arrow" href="<?= $this->url('docs/lifecycle/') ?>">Lifecycle and events</a></p>
            </div>
            <div class="split-code">
                <?= $this->code('php', <<<'PHP'
                    use Nativella\Dialog\FileFilter;
                    use Nativella\Nativella;

                    $native = Nativella::create();

                    $path = $native->dialog->openFile(
                        title: 'Import a note',
                        filters: [new FileFilter('Markdown', ['md'])],
                    );

                    if (null !== $path) {
                        $id = $notes->import($path);

                        $native->notification->send('Imported', basename($path));
                        $native->window->navigate('/notes/'.$id);
                    }
                    PHP, 'src/Import.php') ?>
            </div>
        </div>
    </section>

    <section class="section" id="how-it-works" aria-labelledby="how-title">
        <div class="wrap">
            <div class="section-head">
                <p class="kicker">How it works</p>
                <h2 id="how-title">One process. Two threads. Zero sockets.</h2>
                <p>A Tauri window registers a <code>php://</code> scheme, and every request of that scheme goes to an embedded PHP instead of the network.</p>
            </div>
            <div class="diagram" tabindex="0" aria-label="Diagram, scrollable">
                <?= $architecture ?>
            </div>
            <p class="diagram-hint" aria-hidden="true">Scroll sideways for the whole diagram →</p>
            <ol class="steps">
                <li><strong>The page asks for a URL.</strong> The webview hands the <code>php://localhost/</code> request to Rust. A file of <code>public/</code> comes straight back from disk.</li>
                <li><strong>PHP runs your front controller.</strong> The rest is queued to the PHP thread: a full request through a custom SAPI, with the cookie jar and the redirects WebKit skips for custom schemes.</li>
                <li><strong>PHP calls native code</strong> with <code>nativella_call()</code>, wrapped by the PHP package.</li>
                <li><strong>Native events come back</strong> as internal requests to <code>/_nativella/event</code>, which no page can forge…</li>
                <li><strong>…and reach the page</strong> as <code>nativella:*</code> DOM events.</li>
            </ol>
            <p class="section-more"><a class="link-arrow" href="<?= $this->url('docs/how-it-works/') ?>">The whole story, and what WebKit does not do</a></p>
        </div>
    </section>

    <section class="section section-tinted" id="apis" aria-labelledby="apis-title">
        <div class="wrap">
            <div class="section-head">
                <p class="kicker">Native APIs</p>
                <h2 id="apis-title">Everything a desktop app reaches for.</h2>
            </div>
            <ul class="api-grid">
                <?php foreach ($apis as [$slug, $name, $description, $example]) { ?>
                    <li>
                        <a class="api-card" href="<?= $this->url('docs/'.$slug.'/') ?>">
                            <h3><?= $this->e($name) ?></h3>
                            <p><?= $this->e($description) ?></p>
                            <code><?= $this->e($example) ?></code>
                        </a>
                    </li>
                <?php } ?>
            </ul>
        </div>
    </section>

    <section class="section" aria-labelledby="frameworks-title">
        <div class="wrap split split-wide-code">
            <div class="split-text">
                <p class="kicker">Bring your framework</p>
                <h2 id="frameworks-title">One menu item, three ways.</h2>
                <p>A <em>File › Export…</em> item, set when the app boots. Its click arrives as a <code>MenuItemClicked</code> event: a Symfony listener, a Laravel listener, or three lines at the top of <code>index.php</code>.</p>
                <p>The Symfony bundle and the Laravel package also register the APIs in the container, point the database to the data directory and migrate it at boot.</p>
                <p><a class="link-arrow" href="<?= $this->url('docs/symfony/') ?>">Symfony</a> <a class="link-arrow" href="<?= $this->url('docs/laravel/') ?>">Laravel</a> <a class="link-arrow" href="<?= $this->url('docs/plain-php/') ?>">Plain PHP</a></p>
            </div>
            <div class="tabs">
                <div class="tab-list" role="tablist" aria-label="Framework">
                    <button type="button" role="tab" id="tab-symfony" aria-controls="panel-symfony" aria-selected="true">Symfony</button>
                    <button type="button" role="tab" id="tab-laravel" aria-controls="panel-laravel" aria-selected="false">Laravel</button>
                    <button type="button" role="tab" id="tab-vanilla" aria-controls="panel-vanilla" aria-selected="false">Plain PHP</button>
                </div>
                <div class="tab-panel" role="tabpanel" id="panel-symfony" aria-labelledby="tab-symfony">
                    <?= $this->code('php', <<<'PHP'
                        namespace App\EventListener;

                        use App\Note\NoteExporter;
                        use Nativella\Dialog;
                        use Nativella\Event\MenuItemClicked;
                        use Nativella\Notification;
                        use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

                        #[AsEventListener]
                        final readonly class ExportListener
                        {
                            public function __construct(
                                private Dialog $dialog,
                                private NoteExporter $exporter,
                                private Notification $notification,
                            ) {
                            }

                            public function __invoke(MenuItemClicked $event): void
                            {
                                if ('export' !== $event->id) {
                                    return;
                                }

                                $path = $this->dialog->saveFile(fileName: 'notes.zip');
                                if (null !== $path) {
                                    $this->exporter->export($path);
                                    $this->notification->send('Export finished', $path);
                                }
                            }
                        }
                        PHP, 'src/EventListener/ExportListener.php') ?>
                </div>
                <div class="tab-panel" role="tabpanel" id="panel-laravel" aria-labelledby="tab-laravel" hidden>
                    <?= $this->code('php', <<<'PHP'
                        namespace App\Listeners;

                        use App\Notes\NoteExporter;
                        use Nativella\Event\MenuItemClicked;
                        use Nativella\Laravel\Facades\Dialog;
                        use Nativella\Laravel\Facades\Notification;

                        final readonly class ExportNotes
                        {
                            public function __construct(
                                private NoteExporter $exporter,
                            ) {
                            }

                            public function handle(MenuItemClicked $event): void
                            {
                                if ('export' !== $event->id) {
                                    return;
                                }

                                $path = Dialog::saveFile(fileName: 'notes.zip');
                                if (null !== $path) {
                                    $this->exporter->export($path);
                                    Notification::send('Export finished', $path);
                                }
                            }
                        }
                        PHP, 'app/Listeners/ExportNotes.php') ?>
                </div>
                <div class="tab-panel" role="tabpanel" id="panel-vanilla" aria-labelledby="tab-vanilla" hidden>
                    <?= $this->code('php', <<<'PHP'
                        use App\NoteExporter;
                        use Nativella\Event\EventFactory;
                        use Nativella\Event\MenuItemClicked;
                        use Nativella\Nativella;

                        require dirname(__DIR__).'/vendor/autoload.php';

                        $native = Nativella::create();

                        // Null unless the runtime itself sent the request.
                        if (null !== $event = EventFactory::fromGlobals()) {
                            if ($event instanceof MenuItemClicked && 'export' === $event->id) {
                                $path = $native->dialog->saveFile(fileName: 'notes.zip');
                                if (null !== $path) {
                                    (new NoteExporter($pdo))->export($path);
                                    $native->notification->send('Export finished', $path);
                                }
                            }

                            http_response_code(204);

                            return;
                        }

                        // …the pages of the application, as usual.
                        PHP, 'public/index.php') ?>
                </div>
            </div>
        </div>
    </section>

    <section class="section section-tinted" aria-labelledby="start-title">
        <div class="wrap">
            <div class="section-head">
                <p class="kicker">Getting started</p>
                <h2 id="start-title">From a PHP project to a window, in three steps.</h2>
            </div>
            <ol class="start-steps">
                <li>
                    <div class="start-text">
                        <h3>Build the runtime</h3>
                        <p>Once. Nativella is built from source for now, against the PHP of the system. Ubuntu 26.04 is where it is tested.</p>
                    </div>
                    <?= $this->code('bash', <<<'BASH'
                        sudo apt install libphp8.5-embed php8.5-dev \
                            libclang-dev libwebkit2gtk-4.1-dev \
                            libxdo-dev librsvg2-dev
                        git clone https://github.com/lyrixx/nativella.git
                        cargo install --locked --path nativella/crates/nativella
                        BASH) ?>
                </li>
                <li>
                    <div class="start-text">
                        <h3>Add the package</h3>
                        <p>From a path repository until it reaches Packagist. Add the Symfony bundle or the Laravel package if you use one.</p>
                    </div>
                    <?= $this->code('bash', <<<'BASH'
                        composer require nativella/nativella:^0.1@dev
                        # Symfony
                        composer require nativella/symfony-bundle:^0.1@dev
                        # Laravel
                        composer require nativella/laravel:^0.1@dev
                        BASH) ?>
                </li>
                <li>
                    <div class="start-text">
                        <h3>Run it</h3>
                        <p>A window opens on your home page. When the app is ready, pack it into one executable.</p>
                    </div>
                    <?= $this->code('bash', <<<'BASH'
                        nativella run my-app
                        nativella build my-app
                        ./dist/my-app
                        BASH) ?>
                </li>
            </ol>
            <p class="section-more"><a class="link-arrow" href="<?= $this->url('docs/installation/') ?>">Installation, step by step</a></p>
        </div>
    </section>

    <section class="section" aria-labelledby="demos-title">
        <div class="wrap">
            <div class="section-head">
                <p class="kicker">Demos</p>
                <h2 id="demos-title">Three small apps, one per way of writing PHP.</h2>
                <p>They live in the repository, and run with <code>nativella run demos/&lt;name&gt;</code> after a <code>composer install</code>.</p>
            </div>
            <ul class="demo-grid">
                <li class="demo">
                    <p class="demo-stack">Symfony · Doctrine · Twig</p>
                    <h3>Notes</h3>
                    <p>Markdown notes with an application menu, a tray icon, context menus, a quick-note global shortcut, and files dropped on the window turned into notes.</p>
                    <a href="<?= $this->e($this->site->repository) ?>/tree/main/demos/symfony"><code>demos/symfony</code></a>
                </li>
                <li class="demo">
                    <p class="demo-stack">Laravel · Eloquent · Blade</p>
                    <h3>Tasks</h3>
                    <p>A task list that exports through a save dialog, keeps a count in the tray and comes back to the front with a global shortcut.</p>
                    <a href="<?= $this->e($this->site->repository) ?>/tree/main/demos/laravel"><code>demos/laravel</code></a>
                </li>
                <li class="demo">
                    <p class="demo-stack">Plain PHP · DBAL · no framework</p>
                    <h3>Snippets</h3>
                    <p>Text snippets copied to the clipboard in one click, deleted after a native confirmation, imported from files picked or dropped.</p>
                    <a href="<?= $this->e($this->site->repository) ?>/tree/main/demos/vanilla"><code>demos/vanilla</code></a>
                </li>
            </ul>
        </div>
    </section>

    <section class="section section-tinted section-phone" aria-labelledby="phone-title">
        <div class="wrap split split-phone">
            <div class="split-text">
                <p class="kicker">Android · experimental</p>
                <h2 id="phone-title">The same Notes, on a phone.</h2>
                <p>The Symfony demo, unchanged, built into an APK with <code>mobile/android/build.sh demos/symfony</code> and running on an Android 16 emulator: Doctrine migrations at boot, forms and sessions, Markdown, the clipboard of the phone, and a native dialog opened by PHP. A note takes the location of the phone in one method call; the other demos vibrate, and ask for a fingerprint.</p>
                <p class="phone-note">Early days: arm64 builds only, no iOS yet, and the static PHP inside comes from NativePHP’s builds until Nativella makes its own.</p>
                <p><a class="link-arrow" href="<?= $this->url('docs/mobile/') ?>">Mobile, in the docs</a></p>
            </div>
            <div class="phones">
                <figure class="phone">
                    <img src="<?= $this->asset('android/notes-note.webp') ?>" width="360" height="800" loading="lazy" decoding="async" alt="The Notes demo on an Android phone: a note titled “Hello from Android”, its Markdown rendered as “PHP 8.5 runs in-process.”">
                    <figcaption>Rendered by PHP</figcaption>
                </figure>
                <figure class="phone">
                    <img src="<?= $this->asset('android/notes-copied.webp') ?>" width="360" height="800" loading="lazy" decoding="async" alt="After tapping Copy, the page says “Copied as Markdown.”, and the clipboard preview of Android shows the Markdown source that PHP copied.">
                    <figcaption>Copied by PHP</figcaption>
                </figure>
                <figure class="phone">
                    <img src="<?= $this->asset('android/notes-confirm.webp') ?>" width="360" height="800" loading="lazy" decoding="async" alt="Tapping Delete opens a native Android dialog, “Delete the note”, with Cancel and Delete buttons.">
                    <figcaption>Asked by PHP</figcaption>
                </figure>
            </div>
        </div>
    </section>

    <section class="section section-status" id="status" aria-labelledby="status-title">
        <div class="wrap">
            <div class="section-head">
                <p class="kicker">Status</p>
                <h2 id="status-title">Early, and honest about it.</h2>
                <p>Nativella started as a spike: can a Tauri window talk to PHP without an HTTP server? It can. What it is not yet is finished.</p>
            </div>
            <div class="stats">
                <div class="stat"><strong>0</strong><span>ports opened</span></div>
                <div class="stat"><strong>~5 ms</strong><span>a Symfony page with Twig and a session, kernel booted every time</span></div>
                <div class="stat"><strong>1</strong><span>executable for the whole project</span></div>
            </div>
            <div class="status-columns">
                <div class="status-card status-yes">
                    <h3>Works today</h3>
                    <ul>
                        <li>Symfony, Laravel and plain PHP, front controller untouched</li>
                        <li>Forms, sessions, cookies and redirects</li>
                        <li>Twenty native APIs, nineteen native events</li>
                        <li>SQLite in the data directory, migrated at boot</li>
                        <li>A single executable carrying the project, and its own PHP if you want</li>
                        <li>Android, experimentally: the three demos, in arm64 APKs</li>
                    </ul>
                </div>
                <div class="status-card status-no">
                    <h3>Not yet</h3>
                    <ul>
                        <li>macOS and Windows: written for, never tested; iOS: not built</li>
                        <li>One PHP thread: a dialog holds back every other request</li>
                        <li>No worker mode: the framework boots on every request</li>
                        <li>Installers, code signing and updates</li>
                        <li>No screen readers on Linux, to dodge a WebKitGTK crash</li>
                    </ul>
                </div>
            </div>
            <p class="section-more"><a class="link-arrow" href="<?= $this->url('docs/limitations/') ?>">Limitations and roadmap</a></p>
        </div>
    </section>

    <section class="closing" aria-labelledby="closing-title">
        <div class="wrap closing-inner">
            <h2 id="closing-title">Your next desktop app is already written. In PHP.</h2>
            <div class="hero-actions">
                <a class="button button-primary" href="<?= $this->url('docs/getting-started/') ?>">Get started <span aria-hidden="true">→</span></a>
                <a class="button button-secondary" href="<?= $this->e($this->site->repository) ?>">View on GitHub</a>
            </div>
        </div>
    </section>
</main>
