<?php

namespace Tests\Feature;

use App\Models\Task;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Nativella\Laravel\Facades\Nativella;
use Tests\TestCase;

final class TaskTest extends TestCase
{
    use RefreshDatabase;

    protected function tearDown(): void
    {
        unset($_SERVER['NATIVELLA_PLATFORM']);

        parent::tearDown();
    }

    public function testTheListShowsTheOpenTasksFirst(): void
    {
        Task::factory()->completed()->create(['title' => 'Install Nativella']);
        Task::factory()->create(['title' => 'Water the plants']);
        Task::factory()->create(['title' => 'Call the dentist']);

        $this->get('/')
            ->assertOk()
            ->assertSeeInOrder(['Call the dentist', 'Water the plants', 'Install Nativella'])
            ->assertSee('2 open tasks');
    }

    public function testATaskIsAdded(): void
    {
        $this->post('/tasks', ['title' => 'Water the plants'])->assertRedirect('/');

        $this->assertSame(['Water the plants'], Task::query()->pluck('title')->all());
    }

    public function testATaskNeedsATitle(): void
    {
        $this->from('/')->post('/tasks', ['title' => ''])->assertRedirect('/')->assertSessionHasErrors('title');

        $this->assertSame(0, Task::query()->count());
    }

    public function testATaskIsCompletedThenReopened(): void
    {
        $task = Task::factory()->create();

        $this->post("/tasks/{$task->id}/complete")->assertRedirect('/');
        $this->assertTrue($task->refresh()->isCompleted());

        $this->post("/tasks/{$task->id}/reopen")->assertRedirect('/');
        $this->assertFalse($task->refresh()->isCompleted());
    }

    public function testCompletingTheLastOpenTaskIsCelebrated(): void
    {
        $native = Nativella::fake();
        Task::factory()->completed()->create();
        $task = Task::factory()->create();

        $this->post("/tasks/{$task->id}/complete");

        $this->assertSame([['method' => 'notification.send', 'payload' => ['title' => 'All done!', 'body' => 'Every task is completed. Time for a break.', 'reference' => 'all-done']]], $native->calls('notification.send'));
        $this->assertSame([], $native->calls('haptics.feedback'), 'Computers do not vibrate.');
    }

    public function testOnAPhoneTheLastOpenTaskDoneVibrates(): void
    {
        $native = Nativella::fake();
        // What the runtime sets on Android.
        $_SERVER['NATIVELLA_PLATFORM'] = 'android';
        $task = Task::factory()->create();

        $this->post("/tasks/{$task->id}/complete");

        $this->assertSame([['method' => 'haptics.feedback', 'payload' => ['feedback' => 'success']]], $native->calls('haptics.feedback'));
    }

    public function testCompletingATaskWhileOthersAreOpenIsNotCelebrated(): void
    {
        $native = Nativella::fake();
        Task::factory()->create();
        $task = Task::factory()->create();

        $this->post("/tasks/{$task->id}/complete");
        $task->refresh()->save();

        $this->assertSame([], $native->calls('notification.send'));
    }

    public function testTheTrayCountsTheOpenTasks(): void
    {
        $native = Nativella::fake();
        Task::factory()->create();

        $this->post('/tasks', ['title' => 'Water the plants']);

        $this->assertSame('2 open tasks', last($native->calls('tray.set'))['payload']['tooltip']);
    }

    public function testDeletingAsksForConfirmation(): void
    {
        $native = Nativella::fake();
        $native->respond('dialog.confirm', ['confirmed' => true]);
        $task = Task::factory()->create(['title' => 'Water the plants']);

        $this->delete("/tasks/{$task->id}")->assertRedirect('/')->assertSessionHas('status', '"Water the plants" deleted.');

        $this->assertModelMissing($task);
        $this->assertSame('Delete "Water the plants"?', $native->calls('dialog.confirm')[0]['payload']['message']);
        $this->assertSame('Nothing to do', last($native->calls('tray.set'))['payload']['tooltip']);
    }

    public function testDeletingCanBeCancelled(): void
    {
        Nativella::fake();
        $task = Task::factory()->create();

        $this->delete("/tasks/{$task->id}")->assertRedirect('/')->assertSessionMissing('status');

        $this->assertModelExists($task);
    }

    public function testInABrowserThePageAsksBeforeDeleting(): void
    {
        $task = Task::factory()->create();

        $this->get('/')->assertSee('return confirm(', false);
        $this->delete("/tasks/{$task->id}")->assertRedirect('/');

        $this->assertModelMissing($task);
    }

    public function testInABrowserTheNativeFeaturesAreHidden(): void
    {
        Task::factory()->create();

        $this->get('/')
            ->assertOk()
            ->assertDontSee('Export…')
            ->assertSee('Copy, export and notifications come with the desktop application.');
        $this->post('/tasks/copy')->assertNotFound();
        $this->post('/tasks/export')->assertNotFound();
    }

    public function testInNativellaTheNativeFeaturesAreShown(): void
    {
        Nativella::fake();
        Task::factory()->create();

        $this->get('/')
            ->assertOk()
            ->assertSee('Export…')
            ->assertSee('Copy')
            ->assertDontSee('return confirm(', false);
    }
}
