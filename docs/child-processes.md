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
| `write(string $alias, string $data): void` | Writes to its standard input. It returns at once, and the process reads it when it likes. Throws if no process of that name is running, or if its input is closed. |
| `closeInput(string $alias): void` | Closes its standard input, after what was written. This is the end of the input that `sort`, `wc` or `jq` wait for. Throws if no process of that name is running. |
| `stop(string $alias): void` | Asks it to end (`SIGTERM`), and kills it 3 seconds later if it has not. Nothing happens if it is not running. |
| `isRunning(string $alias): bool` | |
| `all(): list<RunningProcess>` | The running processes: their `alias`, `pid` and `command`. |

`$command` is the program, then its arguments. No shell is involved, so nothing needs quoting, and `|`, `>` or `$VAR` mean nothing. For those, start a shell: `['sh', '-c', 'du -sh ~ | sort -h']`.

The process starts in the project directory, or in `$cwd`, relative to it. It gets the application's environment, plus the Nativella variables that PHP finds in `$_SERVER` (`NATIVELLA_DATA_DIR`, `NATIVELLA_SECRET`, `NATIVELLA_PLATFORM` and `NATIVELLA_VERSION`), plus `$env`, which wins. On Linux, it does not get the `NO_AT_BRIDGE` that Nativella [sets for itself](configuration.md#environment-variables). An application worker opens the same database, with the same secret, as the requests do.

## Output and end

```php
use Nativella\Event\ChildProcessExited;
use Nativella\Event\ChildProcessOutput;
use Symfony\Component\EventDispatcher\Attribute\AsEventListener;

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

These are Symfony listeners. [Lifecycle and events](lifecycle.md#handling-events) says how the other integrations route events.

`ChildProcessOutput::$output` is what the process wrote, as it came: often whole lines with their line endings, sometimes a prompt or a part of a line, never half a character. `ChildProcessExited` comes after the process's last output, however long PHP takes to handle it. A daemon that the process started outside of its process group may keep the output open after the process has ended. Nativella then waits for 2 seconds of quiet, and no longer.

Both carry the `pid`. A process started again under the same alias, after a `stop()` or from a `ChildProcessExited` listener, may run before the previous one's events are handled. Compare the `pid` to tell them apart.

Every event is a request to PHP, on its single thread. Nativella sends one output event at a time per stream. Until PHP has handled it, what the process writes waits in the pipe, and a process that writes faster than PHP reads waits too. Keep processes quiet, or have them write to a file.

## PHP processes

The `nativella` command embeds PHP as a library, not as a command, so running `php bin/console` needs PHP's command line on the machine, which a [standalone executable](building.md) built with static PHP does not ask for. Prefer a [timer](timers.md) for periodic work, because it runs in the application itself.

## When the application quits

Every process gets a `SIGTERM`, then 3 seconds to end before it is killed. A worker that handles `SIGTERM` finishes its job first, as under a process manager. No process starts any more, so a `ChildProcessExited` listener that starts its worker again does not keep it alive.

Each process leads its own process group, which ends with it. `['sh', '-c', 'a | b']` ends with `a` and `b`, not only `sh`, and so does what a process leaves running behind it.

On Linux, processes end with the application even when it does not quit properly, after a crash or a `kill -9`. Ctrl+C, a closed terminal or `kill` quit it properly.

On Windows, there is no `SIGTERM` and no group, so `stop()` kills the process right away, and only it.

`ChildProcess` is desktop only. On a phone, it throws a `NativeCallException`.
