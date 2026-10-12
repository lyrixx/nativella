# Child processes

`Nativella\ChildProcess` starts processes and watches them: a queue worker, a command line tool, a local server. What they write reaches PHP as events, and so does their end. They end with the application.

```php
use Nativella\ChildProcess;

$processes->start('worker', ['php', 'bin/console', 'messenger:consume', 'async']);

$processes->isRunning('worker'); // true
$processes->stop('worker');
```

| Method | |
|---|---|
| `start(string $alias, array $command, ?string $cwd = null, array $env = []): int` | Starts `$command` under the name `$alias`, and returns its process id. Throws if a process of that name is running; one being stopped gets 4 seconds to end first, so `stop()` then `start()` restarts it. |
| `write(string $alias, string $data): void` | Writes to its standard input. It returns at once: the process reads it when it likes. |
| `closeInput(string $alias): void` | Closes its standard input, after what was written: the end of the input that `sort`, `wc` or `jq` wait for. |
| `stop(string $alias): void` | Asks it to end (`SIGTERM`), and kills it 3 seconds later if it has not. Nothing happens if it is not running. |
| `isRunning(string $alias): bool` | |
| `all(): list<RunningProcess>` | The running processes: their `alias`, `pid` and `command`. |

`$command` is the program, then its arguments: no shell is involved, so nothing needs quoting, and `|`, `>` or `$VAR` mean nothing. For those, start a shell: `['sh', '-c', 'du -sh ~ | sort -h']`.

The process starts in the project directory, or in `$cwd`, relative to it. It gets the environment of the application, plus `$env`.

## Output and end

```php
use Nativella\Event\ChildProcessExited;
use Nativella\Event\ChildProcessOutput;

#[AsEventListener]
public function onOutput(ChildProcessOutput $event): void
{
    // $event->alias, $event->pid, $event->stream (Stream::Stdout or Stream::Stderr), $event->output
    $this->logger->info($event->output, ['process' => $event->alias]);
}

#[AsEventListener]
public function onExit(ChildProcessExited $event): void
{
    // $event->code: the exit code, null when a signal ended the process
}
```

`ChildProcessOutput::$output` is what the process wrote, as it came: often whole lines with their line endings, sometimes a prompt or a part of a line, never half a character. `ChildProcessExited` comes after the last output of the process.

Both carry the `pid`. A process started again under the same alias, after a `stop()` or from a listener of `ChildProcessExited`, may run before the events of the previous one are handled: compare the `pid` to tell them apart.

Every event is a request to PHP, on its single thread. Nativella sends one output event at a time per stream: until PHP has handled it, what the process writes waits in the pipe, and a process that writes faster than PHP reads waits too. Keep processes quiet, or have them write to a file.

## PHP processes

The `nativella` command embeds PHP as a library, not as a command: running `php bin/console` needs PHP's command line on the machine, which a [standalone executable](building.md) built with static PHP does not ask for. Prefer a [timer](timers.md) for periodic work: it runs in the application itself.

## When the application quits

Every process gets a `SIGTERM`, then 3 seconds to end before it is killed. A worker that handles `SIGTERM` finishes its job first, as under a process manager. No process starts any more, so a listener of `ChildProcessExited` that starts its worker again does not keep it alive.

Each process leads a process group of its own, which ends with it: `['sh', '-c', 'a | b']` ends with `a` and `b`, not only `sh`, and so does what a process leaves running behind it.

On Linux, processes end with the application even when it does not quit properly: a crash, `kill -9`. Ctrl+C, a closed terminal or `kill` quit it properly.

On Windows, there is no `SIGTERM` and no group: `stop()` kills the process right away, and only it.

Desktop only: on a phone, `ChildProcess` throws a `NativeCallException`.
