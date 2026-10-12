<?php

namespace Tests\Feature;

use App\Models\Task;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Log;
use Nativella\Laravel\Facades\Nativella;
use Tests\TestCase;

final class ExportTasksTest extends TestCase
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
        if (is_file($this->file)) {
            unlink($this->file);
        }

        parent::tearDown();
    }

    public function testTheTasksAreWrittenWhereTheUserWants(): void
    {
        $native = Nativella::fake();
        $native->respond('dialog.saveFile', ['path' => $this->file]);
        Task::factory()->create(['title' => 'Water the plants']);

        $this->post('/tasks/export')->assertRedirect('/')->assertSessionHas('status', "Tasks exported to {$this->file}.");

        $this->assertStringEqualsFile($this->file, "# Tasks\n\n- [ ] Water the plants\n");
        $this->assertSame([
            'title' => 'Export the tasks',
            'directory' => null,
            'fileName' => 'tasks.md',
            'filters' => [['name' => 'Markdown', 'extensions' => ['md']]],
        ], $native->calls('dialog.saveFile')[0]['payload']);
    }

    public function testTheUserCanCancel(): void
    {
        Nativella::fake();

        $this->post('/tasks/export')->assertRedirect('/')->assertSessionMissing(['status', 'error']);

        $this->assertFileDoesNotExist($this->file);
    }

    public function testAFileThatCannotBeWrittenIsReported(): void
    {
        $missing = storage_path('framework/testing/missing/tasks.md');
        Nativella::fake()->respond('dialog.saveFile', ['path' => $missing]);
        $log = Log::spy();

        $this->post('/tasks/export')->assertRedirect('/')->assertSessionHas('error', "The tasks could not be written to \"{$missing}\".");

        $log->shouldHaveReceived('warning')->once();
    }
}
