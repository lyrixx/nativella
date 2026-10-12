<?php

namespace Tests\Feature;

use App\Models\Task;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Nativella\Laravel\Facades\Nativella;
use Tests\TestCase;

final class CopyTasksTest extends TestCase
{
    use RefreshDatabase;

    public function testTheTasksAreCopiedAsMarkdown(): void
    {
        $native = Nativella::fake();
        Task::factory()->completed()->create(['title' => 'Install Nativella']);
        Task::factory()->create(['title' => 'Water the plants']);

        $this->post('/tasks/copy')->assertRedirect('/')->assertSessionHas('status', '2 tasks copied to the clipboard.');

        $this->assertSame(
            [['method' => 'clipboard.writeText', 'payload' => ['text' => "# Tasks\n\n- [ ] Water the plants\n- [x] Install Nativella\n"]]],
            $native->calls('clipboard.writeText'),
        );
    }
}
