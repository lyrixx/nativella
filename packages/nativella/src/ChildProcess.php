<?php

namespace Nativella;

use Nativella\ChildProcess\RunningProcess;

/**
 * Processes the application starts and watches: a queue worker, a command
 * line tool, a local server. What they write comes as
 * Event\ChildProcessOutput events, their end as Event\ChildProcessExited.
 *
 * They end with the application. Desktop only.
 */
final readonly class ChildProcess
{
    public function __construct(
        private Client $client,
    ) {
    }

    /**
     * Starts $command, under the name $alias, and returns its process id. A
     * process of that name being stopped gets 4 seconds to end first.
     *
     * @param non-empty-list<string> $command the program, then its arguments: no shell is involved
     * @param string|null            $cwd     relative to the project, the project itself by default
     * @param array<string, string>  $env     on top of the environment of the application
     */
    public function start(string $alias, array $command, ?string $cwd = null, array $env = []): int
    {
        return $this->client->call('childProcess.start', [
            'alias' => $alias,
            'command' => $command,
            'cwd' => $cwd,
            // A JSON object, even empty.
            'env' => (object) $env,
        ])['pid'];
    }

    /**
     * Writes to the standard input of the process. It returns at once: the
     * process reads it when it likes.
     */
    public function write(string $alias, string $data): void
    {
        $this->client->call('childProcess.write', ['alias' => $alias, 'data' => $data]);
    }

    /**
     * Closes the standard input of the process, after what was written: the
     * end that `sort` or `jq` wait for.
     */
    public function closeInput(string $alias): void
    {
        $this->client->call('childProcess.closeInput', ['alias' => $alias]);
    }

    /**
     * Asks the process to end (SIGTERM), and kills it 3 seconds later if it
     * has not. Nothing happens if it is not running.
     */
    public function stop(string $alias): void
    {
        $this->client->call('childProcess.stop', ['alias' => $alias]);
    }

    public function isRunning(string $alias): bool
    {
        foreach ($this->all() as $process) {
            if ($alias === $process->alias) {
                return true;
            }
        }

        return false;
    }

    /**
     * @return list<RunningProcess>
     */
    public function all(): array
    {
        // @phpstan-ignore return.type (a JSON array, so a list: crates/nativella/src/native/child_process.rs)
        return array_map(RunningProcess::fromArray(...), $this->client->call('childProcess.all')['processes']);
    }
}
