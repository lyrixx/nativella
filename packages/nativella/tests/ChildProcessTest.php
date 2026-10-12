<?php

namespace Nativella\Tests;

use Nativella\ChildProcess;
use Nativella\ChildProcess\RunningProcess;
use Nativella\ChildProcess\Stream;
use Nativella\Client\FakeClient;
use Nativella\Event\ChildProcessExited;
use Nativella\Event\ChildProcessOutput;
use Nativella\Event\EventFactory;
use PHPUnit\Framework\TestCase;

final class ChildProcessTest extends TestCase
{
    public function testAProcessIsStartedFedAndStopped(): void
    {
        $client = new FakeClient();
        $processes = new ChildProcess($client);
        $client->respond('childProcess.start', ['pid' => 4242]);

        $this->assertSame(4242, $processes->start('worker', ['php', 'bin/console', 'messenger:consume'], env: ['APP_ENV' => 'prod']));
        $processes->write('worker', "ping\n");
        $processes->closeInput('worker');
        $processes->stop('worker');

        $this->assertEquals([
            ['method' => 'childProcess.start', 'payload' => ['alias' => 'worker', 'command' => ['php', 'bin/console', 'messenger:consume'], 'cwd' => null, 'env' => (object) ['APP_ENV' => 'prod']]],
            ['method' => 'childProcess.write', 'payload' => ['alias' => 'worker', 'data' => "ping\n"]],
            ['method' => 'childProcess.closeInput', 'payload' => ['alias' => 'worker']],
            ['method' => 'childProcess.stop', 'payload' => ['alias' => 'worker']],
        ], $client->calls());
    }

    public function testNoVariableIsStillAJsonObject(): void
    {
        $client = new FakeClient();

        new ChildProcess($client)->start('clock', ['date']);

        $this->assertSame('{}', json_encode($client->calls('childProcess.start')[0]['payload']['env']));
    }

    public function testTheRunningProcesses(): void
    {
        $client = new FakeClient();
        $processes = new ChildProcess($client);
        $client->respond('childProcess.all', ['processes' => [['alias' => 'worker', 'pid' => 4242, 'command' => ['php', 'worker.php']]]]);
        $client->respond('childProcess.all', ['processes' => []]);

        $this->assertEquals([new RunningProcess('worker', 4242, ['php', 'worker.php'])], $processes->all());
        $this->assertFalse($processes->isRunning('worker'));
    }

    public function testTheEventsOfAProcess(): void
    {
        $this->assertEquals(new ChildProcessOutput('worker', 4242, Stream::Stderr, "Oops\n"), EventFactory::create('childProcess.output', ['alias' => 'worker', 'pid' => 4242, 'stream' => 'stderr', 'output' => "Oops\n"]));
        $this->assertEquals(new ChildProcessExited('worker', 4242, null), EventFactory::create('childProcess.exited', ['alias' => 'worker', 'pid' => 4242, 'code' => null]));
    }
}
