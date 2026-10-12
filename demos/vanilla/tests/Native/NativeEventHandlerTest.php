<?php

namespace App\Tests\Native;

use App\Import\SnippetImporter;
use App\Native\NativeEventHandler;
use App\Snippet\SnippetRepository;
use App\Snippet\SnippetSchema;
use Doctrine\DBAL\Connection;
use Doctrine\DBAL\DriverManager;
use Nativella\Client\FakeClient;
use Nativella\Event\AppBooted;
use Nativella\Event\EventFactory;
use Nativella\Event\GlobalShortcutPressed;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\WindowFocused;
use Nativella\Exception\NativeCallException;
use Nativella\Nativella;
use PHPUnit\Framework\TestCase;

final class NativeEventHandlerTest extends TestCase
{
    private const string FIXTURES = __DIR__.'/../Fixtures';

    private FakeClient $client;
    private Connection $connection;
    private NativeEventHandler $handler;

    protected function setUp(): void
    {
        $this->client = new FakeClient();
        $this->connection = DriverManager::getConnection(['driver' => 'pdo_sqlite', 'memory' => true]);
        $native = new Nativella($this->client);
        $importer = new SnippetImporter($native, new SnippetRepository($this->connection));

        $this->handler = new NativeEventHandler($importer, $native, new SnippetSchema($this->connection));
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);
    }

    public function testBootCreatesTheDatabase(): void
    {
        $this->handler->handle(new AppBooted());

        $this->assertTrue($this->connection->createSchemaManager()->tableExists(SnippetSchema::TABLE));
    }

    public function testBootSetsTheMenuThenRegistersTheShortcut(): void
    {
        $this->handler->handle(new AppBooted());

        $calls = $this->client->calls();
        $this->assertSame(['menu.set', 'globalShortcut.register'], \array_slice(array_column($calls, 'method'), -2));
        $this->assertSame(['accelerator' => 'CmdOrCtrl+Shift+Space'], $this->client->calls('globalShortcut.register')[0]['payload']);
    }

    public function testTheMenuHasFileEditWindowAndHelp(): void
    {
        $this->handler->handle(new AppBooted());

        $menu = $this->client->calls('menu.set')[0]['payload']['items'];
        $submenus = array_column($menu, 'items', 'label');
        $this->assertSame(['File', 'Edit', 'Window', 'Help'], \array_slice(array_keys($submenus), -4));

        $file = $submenus['File'];
        $this->assertSame(
            ['type' => 'action', 'id' => 'new-snippet', 'label' => 'New snippet', 'accelerator' => 'CmdOrCtrl+N', 'enabled' => true, 'url' => '/snippets/new'],
            $file[0],
        );
        $this->assertSame(
            ['type' => 'action', 'id' => 'import', 'label' => 'Import…', 'accelerator' => 'CmdOrCtrl+O', 'enabled' => true],
            $file[1],
        );
        $this->assertContains(['type' => 'role', 'role' => 'copy'], $submenus['Edit']);
        $this->assertContains(['type' => 'role', 'role' => 'minimize'], $submenus['Window']);
    }

    public function testQuitEndsTheFileMenu(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'linux';

        $this->handler->handle(new AppBooted());

        $file = array_column($this->client->calls('menu.set')[0]['payload']['items'], 'items', 'label')['File'];
        $this->assertSame([['type' => 'separator'], ['type' => 'role', 'role' => 'quit']], \array_slice($file, -2));
    }

    public function testOnMacOsTheApplicationMenuComesFirst(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'macos';

        $this->handler->handle(new AppBooted());

        $menu = $this->client->calls('menu.set')[0]['payload']['items'];
        // Named after the application, as app.info gives it.
        $this->assertSame('Nativella', $menu[0]['label']);
        $this->assertContains(['type' => 'role', 'role' => 'quit'], $menu[0]['items']);
        $this->assertNotContains(['type' => 'role', 'role' => 'quit'], array_column($menu, 'items', 'label')['File']);
    }

    public function testOnMobileBootOnlyMigrates(): void
    {
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        $this->handler->handle(new AppBooted());

        $this->assertSame([], $this->client->calls());
        $this->assertTrue($this->connection->createSchemaManager()->tableExists(SnippetSchema::TABLE));
    }

    public function testBootGoesOnWhenTheShortcutIsTaken(): void
    {
        $this->client->respond('globalShortcut.register', new NativeCallException('globalShortcut.register', new \RuntimeException('HotKey already registered.')));

        $this->expectErrorLog();
        $this->handler->handle(new AppBooted());

        $this->assertCount(1, $this->client->calls('menu.set'));
    }

    public function testTheShortcutFocusesTheWindow(): void
    {
        $this->handler->handle(new GlobalShortcutPressed(NativeEventHandler::FOCUS_SHORTCUT));

        $this->assertSame([['method' => 'window.focus', 'payload' => ['window' => null]]], $this->client->calls());
    }

    public function testOtherShortcutsAreIgnored(): void
    {
        $this->handler->handle(new GlobalShortcutPressed('CmdOrCtrl+Shift+K'));

        $this->assertSame([], $this->client->calls());
    }

    public function testImportFromTheMenuRefreshesThePages(): void
    {
        $this->handler->handle(new AppBooted());
        $this->client->reset();
        $this->client->respond('dialog.openFile', ['path' => self::FIXTURES.'/hello.txt']);

        $this->handler->handle(new MenuItemClicked(NativeEventHandler::MENU_IMPORT, null));

        $this->assertSame(['dialog.openFile', 'notification.send', 'page.dispatch'], array_column($this->client->calls(), 'method'));
        $this->assertEquals(
            ['event' => 'snippets.imported', 'detail' => (object) ['message' => 'Imported “hello.txt”'], 'window' => null],
            $this->client->calls('page.dispatch')[0]['payload'],
        );
    }

    public function testACancelledImportChangesNothing(): void
    {
        $this->handler->handle(new MenuItemClicked(NativeEventHandler::MENU_IMPORT, null));

        $this->assertSame(['dialog.openFile'], array_column($this->client->calls(), 'method'));
    }

    public function testLinksOfTheMenuNeedNothingFromPhp(): void
    {
        // The runtime opens their URL itself.
        $this->handler->handle(new MenuItemClicked(NativeEventHandler::MENU_NEW_SNIPPET, null));

        $this->assertSame([], $this->client->calls());
    }

    public function testDroppedFilesAreImported(): void
    {
        $this->handler->handle(new AppBooted());
        $this->client->reset();

        $this->handler->handle(EventFactory::create('window.filesDropped', [
            'window' => 'main',
            'paths' => [self::FIXTURES.'/hello.txt', self::FIXTURES.'/image.png'],
        ]));

        $this->assertSame(['hello.txt'], $this->connection->fetchFirstColumn('SELECT title FROM snippet'));
        $this->assertSame(['notification.send', 'page.dispatch'], array_column($this->client->calls(), 'method'));
        $this->assertEquals(
            (object) ['message' => "Imported “hello.txt”\nimage.png: The content must be UTF-8 text."],
            $this->client->calls('page.dispatch')[0]['payload']['detail'],
        );
    }

    public function testOtherEventsAreIgnored(): void
    {
        $this->handler->handle(new WindowFocused('main'));
        $this->handler->handle(EventFactory::create('some.future.event', []));

        $this->assertSame([], $this->client->calls());
    }
}
