<?php

namespace Tests\Unit;

use App\Models\Task;
use App\Models\TaskCollection;
use Tests\TestCase;

final class TaskCollectionTest extends TestCase
{
    public function testTheTasksBecomeAMarkdownChecklist(): void
    {
        $done = new Task(['title' => 'Install Nativella']);
        $done->complete();

        $tasks = new TaskCollection([new Task(['title' => 'Water the plants']), $done]);

        $this->assertSame("# Tasks\n\n- [ ] Water the plants\n- [x] Install Nativella\n", $tasks->toMarkdown());
    }

    public function testNoTaskIsAnEmptyChecklist(): void
    {
        $this->assertSame("# Tasks\n\n", new TaskCollection()->toMarkdown());
    }
}
