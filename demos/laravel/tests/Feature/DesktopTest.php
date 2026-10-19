<?php

namespace Tests\Feature;

use App\Desktop\Shortcut;
use App\Models\Task;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Log;
use Nativella\Event\AppBooted;
use Nativella\Event\GlobalShortcutPressed;
use Nativella\Event\MenuItemClicked;
use Nativella\Event\NotificationClicked;
use Nativella\Exception\NativeCallException;
use Nativella\Laravel\Facades\Nativella;
use Tests\TestCase;

/**
 * What happens on the native side: the application starts, the user picks a
 * menu item, presses a global shortcut.
 */
final class DesktopTest extends TestCase
{
    use RefreshDatabase;

    private string $file;

    protected function setUp(): void
    {
        parent::setUp();

        $this->file = storage_path('framework/testing/tasks-'.bin2hex(random_bytes(4)).'.md');
    }

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_INTERNAL'], $_SERVER['NATIVELLA_PLATFORM']);
        if (is_file($this->file)) {
            unlink($this->file);
        }

        parent::tearDown();
    }

    public function testTheRuntimeStartsTheApplication(): void
    {
        $native = Nativella::fake();
        // What the runtime sets for its own requests.
        $_SERVER['NATIVELLA_INTERNAL'] = '1';

        $this->postJson('/_nativella/event', ['name' => 'app.booted', 'payload' => []])->assertNoContent();

        // In any order: Laravel discovers the listeners in the order of the
        // filesystem, which differs from one machine to another.
        $this->assertEqualsCanonicalizing(['menu.set', 'tray.set', 'globalShortcut.register'], array_column($native->calls(), 'method'));
    }

    public function testTheMenuHasAFileMenu(): void
    {
        $native = Nativella::fake();

        event(new AppBooted());

        $this->assertContains([
            'type' => 'submenu',
            'label' => 'File',
            'items' => [
                ['type' => 'action', 'id' => 'tasks.new', 'label' => 'New task', 'accelerator' => 'CmdOrCtrl+N', 'enabled' => true, 'url' => '/'],
                ['type' => 'action', 'id' => 'tasks.export', 'label' => 'Export…', 'accelerator' => 'CmdOrCtrl+E', 'enabled' => true],
                ['type' => 'separator'],
                ['type' => 'action', 'id' => 'settings', 'label' => 'Settings…', 'accelerator' => 'CmdOrCtrl+,', 'enabled' => true, 'url' => '/settings'],
                ['type' => 'separator'],
                ['type' => 'role', 'role' => 'quit'],
            ],
            'enabled' => true,
        ], $native->calls('menu.set')[0]['payload']['items']);
    }

    public function testTheTrayCountsTheOpenTasks(): void
    {
        $native = Nativella::fake();
        Task::factory()->count(2)->create();
        Task::factory()->completed()->create();
        $native->reset();

        event(new AppBooted());

        $this->assertSame('2 open tasks', $native->calls('tray.set')[0]['payload']['tooltip']);
    }

    public function testTheShortcutIsRegistered(): void
    {
        $native = Nativella::fake();

        event(new AppBooted());

        $this->assertSame([['method' => 'globalShortcut.register', 'payload' => ['accelerator' => 'CmdOrCtrl+Alt+Shift+T']]], $native->calls('globalShortcut.register'));
    }

    public function testAShortcutTakenByAnotherApplicationIsSkipped(): void
    {
        $native = Nativella::fake();
        $native->respond('globalShortcut.register', new NativeCallException('globalShortcut.register', new \RuntimeException('HotKey already registered.')));
        $log = Log::spy();

        event(new AppBooted());

        $log->shouldHaveReceived('warning')->once();
    }

    public function testPhonesHaveNoMenuTrayOrGlobalShortcut(): void
    {
        $native = Nativella::fake();
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';

        event(new AppBooted());
        Task::factory()->create();

        $this->assertSame([], $native->calls());
    }

    public function testANotificationClickBringsTheWindowBack(): void
    {
        $native = Nativella::fake();

        event(new NotificationClicked('all-done'));

        $this->assertSame(['window.open'], array_column($native->calls(), 'method'));
    }

    public function testTheShortcutBringsTheWindowBack(): void
    {
        $native = Nativella::fake();

        event(new GlobalShortcutPressed(Shortcut::ShowWindow->value));

        // The runtime opens it with the options of extra.nativella.window.
        $this->assertSame([['method' => 'window.open', 'payload' => ['label' => 'main']]], $native->calls());
    }

    public function testTheTrayBringsTheWindowBack(): void
    {
        $native = Nativella::fake();

        event(new MenuItemClicked('window.show', null));

        $this->assertSame(['window.open'], array_column($native->calls(), 'method'));
    }

    public function testTheMenuExportsTheTasks(): void
    {
        $native = Nativella::fake();
        $native->respond('dialog.saveFile', ['path' => $this->file]);

        event(new MenuItemClicked('tasks.export', null));

        $this->assertFileExists($this->file);
        $this->assertSame(['title' => 'Tasks exported', 'body' => $this->file, 'reference' => null], $native->calls('notification.send')[0]['payload']);
    }

    public function testAFailedExportFromTheMenuIsShown(): void
    {
        $native = Nativella::fake();
        $native->respond('dialog.saveFile', ['path' => storage_path('framework/testing/missing/tasks.md')]);

        event(new MenuItemClicked('tasks.export', null));

        $this->assertSame('error', $native->calls('dialog.message')[0]['payload']['kind']);
    }

    public function testALinkNeedsNothingMore(): void
    {
        $native = Nativella::fake();

        event(new MenuItemClicked('tasks.new', null));

        $this->assertSame([], $native->calls());
    }
}
