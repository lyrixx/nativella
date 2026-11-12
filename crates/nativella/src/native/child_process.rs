//! `childProcess.*`: processes the application starts and watches, a queue
//! worker or a command line tool. What they write and how they end reach PHP
//! as `childProcess.output` and `childProcess.exited` events.
//!
//! Each process leads a process group of its own, which ends with it:
//! `sh -c 'a | b'` ends with `a` and `b`, and so does what `sh` left behind.
//! They all end with the application, even when it is killed (Linux).

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use super::{Call, NativeError, State, invalid_payload, lock, not_found};
use crate::events;

/// How long a process may take to end once asked, before it is killed.
const GRACE: Duration = Duration::from_secs(3);
/// How often the watcher looks at its process: std cannot wait for a process
/// and kill it at once.
const POLL: Duration = Duration::from_millis(50);
/// How long the pipes of an ended process may stay quiet before its last
/// output: a process that left children behind keeps them open.
const LAST_WORDS: Duration = Duration::from_secs(2);

/// The running processes, by alias.
pub(crate) struct ChildProcesses {
    running: Mutex<HashMap<String, Running>>,
    /// Set when the application quits: no process starts any more.
    closing: AtomicBool,
    /// Where processes start, unless told otherwise.
    project_dir: PathBuf,
    /// The variables of Nativella PHP finds in `$_SERVER`, in the
    /// environment of the processes: a worker finds the data directory and
    /// the secret of the application.
    variables: Vec<(&'static str, String)>,
}

struct Running {
    pid: u32,
    command: Vec<String>,
    /// To the thread that writes to its standard input: a process that does
    /// not read must not block PHP. None once closed.
    stdin: Option<mpsc::Sender<Vec<u8>>>,
    /// Setting it asks the watcher to end the process.
    stop: Arc<AtomicBool>,
}

/// What the watcher of a process gets once the process started.
struct Watched {
    child: Child,
    readers: Readers,
}

/// The threads that hand the output of a process to PHP, one per pipe.
struct Readers {
    /// Disconnected when they are all done.
    done: mpsc::Receiver<()>,
    /// What each one gets.
    done_sender: mpsc::Sender<()>,
    /// How many outputs wait for PHP to handle them.
    delivering: Arc<AtomicUsize>,
}

impl Readers {
    fn new() -> Self {
        let (done_sender, done) = mpsc::channel();

        Self {
            done,
            done_sender,
            delivering: Arc::default(),
        }
    }

    /// Hands what `pipe` gives to `deliver`, as it comes: one call for what
    /// one read gets, a burst of lines or a prompt without its line ending,
    /// never half a character. `deliver` returns once PHP handled it: until
    /// then, the pipe fills and the process waits to write more.
    fn forward(
        &self,
        name: String,
        mut pipe: impl Read + Send + 'static,
        deliver: impl Fn(&[u8]) + Send + 'static,
    ) -> io::Result<()> {
        let done = self.done_sender.clone();
        let delivering = Arc::clone(&self.delivering);

        thread::Builder::new().name(name.clone()).spawn(move || {
            // Dropped with the thread: the watcher waits for it.
            let _done = done;
            let send = |output: &[u8]| {
                if output.is_empty() {
                    return;
                }
                delivering.fetch_add(1, Ordering::SeqCst);
                deliver(output);
                delivering.fetch_sub(1, Ordering::SeqCst);
            };
            let mut buffer = vec![0; 64 * 1024];
            // A character cut in two by a read waits for its end.
            let mut pending = Vec::new();
            loop {
                match pipe.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => pending.extend_from_slice(&buffer[..read]),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        eprintln!("[process] Cannot read the {name}: {error}.");
                        break;
                    }
                }
                let complete = match std::str::from_utf8(&pending) {
                    Ok(_) => pending.len(),
                    // Cut at the end; invalid bytes elsewhere go as U+FFFD.
                    Err(error) if error.error_len().is_none() => error.valid_up_to(),
                    Err(_) => pending.len(),
                };
                let output: Vec<u8> = pending.drain(..complete).collect();
                send(&output);
            }
            send(&pending);
        })?;

        Ok(())
    }

    /// Waits for the last output of an ended process to reach PHP. The time
    /// PHP takes to handle it does not count, only how long the pipes stay
    /// quiet: `quiet` at most.
    fn wait(self, quiet: Duration) {
        drop(self.done_sender);
        let mut quiet_since = Instant::now();
        loop {
            match self.done.recv_timeout(POLL) {
                Err(RecvTimeoutError::Timeout) if self.delivering.load(Ordering::SeqCst) > 0 => {
                    quiet_since = Instant::now();
                }
                Err(RecvTimeoutError::Timeout) if quiet_since.elapsed() < quiet => {}
                // Done, or a pipe that something left behind keeps open.
                _ => return,
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Start {
    alias: String,
    command: Vec<String>,
    cwd: Option<PathBuf>,
    env: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    alias: String,
    data: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Alias {
    alias: String,
}

impl ChildProcesses {
    pub(crate) fn new(project_dir: PathBuf, variables: Vec<(&'static str, String)>) -> Self {
        Self {
            running: Mutex::default(),
            closing: AtomicBool::new(false),
            project_dir,
            variables,
        }
    }

    /// Ends every process, and waits for them a little: the application
    /// quits. A listener of `childProcess.exited` cannot start them again.
    pub(crate) fn stop_all(&self) {
        // Before the lock: a start that holds it now is flagged below.
        self.closing.store(true, Ordering::Relaxed);
        for running in lock(&self.running).values() {
            running.stop.store(true, Ordering::Relaxed);
        }

        let deadline = Instant::now() + GRACE + Duration::from_secs(1);
        while !lock(&self.running).is_empty() && Instant::now() < deadline {
            thread::sleep(POLL);
        }
    }

    /// The command of a process: `env` adds to the environment of the
    /// application, and wins over it.
    fn command(
        &self,
        program: &str,
        arguments: &[String],
        cwd: &Path,
        env: &HashMap<String, String>,
    ) -> Command {
        let mut command = Command::new(program);
        command
            .args(arguments)
            .current_dir(cwd)
            .envs(self.variables.iter().cloned());
        #[cfg(target_os = "linux")]
        if crate::sets_no_at_bridge() {
            command
                .env_remove("NO_AT_BRIDGE")
                .env_remove(crate::OWN_NO_AT_BRIDGE);
        }
        command
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::process::CommandExt;
            let application = std::process::id();
            // SAFETY: prctl() and getppid() are async-signal-safe system calls,
            // and the closure allocates nothing.
            unsafe {
                command.pre_exec(move || {
                    // Its end with the application, even a killed one: the signal
                    // comes when the thread that started it ends, the PHP thread,
                    // which lives as long as the application.
                    if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    // Gone already: the signal would never come.
                    if u32::try_from(libc::getppid()) != Ok(application) {
                        return Err(io::Error::other("the application is gone"));
                    }
                    Ok(())
                });
            }
        }

        command
    }
}

pub(crate) fn call(call: &Call<'_>, method: &str) -> Option<Result<Value, NativeError>> {
    let processes = &call.state().child_processes;

    Some(match method {
        "start" => call
            .payload::<Start>()
            .and_then(|payload| start(call.app, processes, payload)),
        // Queued: the process reads it when it likes.
        "write" => call.payload::<Input>().and_then(|payload| {
            let stdin = lock(&processes.running)
                .get(&payload.alias)
                .map(|running| running.stdin.clone())
                .ok_or_else(|| not_running(&payload.alias))?;
            stdin
                .ok_or(())
                .and_then(|stdin| stdin.send(payload.data.into_bytes()).map_err(|_| ()))
                .map_err(|()| format!("The input of \"{}\" is closed.", payload.alias))?;

            Ok(json!({}))
        }),
        // The end of the input, which `sort` or `jq` wait for: the writer
        // ends once it wrote what came before.
        "closeInput" => call.payload::<Alias>().and_then(|payload| {
            lock(&processes.running)
                .get_mut(&payload.alias)
                .ok_or_else(|| not_running(&payload.alias))?
                .stdin = None;

            Ok(json!({}))
        }),
        "stop" => call.payload::<Alias>().map(|payload| {
            // Stopping a process that already ended is no failure.
            if let Some(running) = lock(&processes.running).get(&payload.alias) {
                running.stop.store(true, Ordering::Relaxed);
            }
            json!({})
        }),
        "all" => Ok(json!({
            "processes": lock(&processes.running)
                .iter()
                .map(|(alias, running)| json!({ "alias": alias, "pid": running.pid, "command": running.command }))
                .collect::<Vec<_>>(),
        })),
        _ => return None,
    })
}

fn start(
    app: &AppHandle,
    processes: &ChildProcesses,
    payload: Start,
) -> Result<Value, NativeError> {
    let Some((program, arguments)) = payload.command.split_first() else {
        return Err(invalid_payload(
            "A process needs a command: the program, then its arguments.",
        ));
    };
    let alias = payload.alias;
    let cwd = match &payload.cwd {
        Some(cwd) => processes.project_dir.join(cwd),
        None => processes.project_dir.clone(),
    };

    // Locked from the check to the insertion: two starts of the same alias
    // would both pass the check otherwise. A process of that name being
    // stopped gets the time to end: stop() then start() restarts it.
    let deadline = Instant::now() + GRACE + Duration::from_secs(1);
    let mut running = loop {
        let running = lock(&processes.running);
        match running.get(&alias) {
            None => break running,
            Some(existing)
                if existing.stop.load(Ordering::Relaxed) && Instant::now() < deadline =>
            {
                drop(running);
                thread::sleep(POLL);
            }
            Some(_) => {
                return Err(format!(
                    "A process named \"{alias}\" is already running: stop it first."
                )
                .into());
            }
        }
    };
    if processes.closing.load(Ordering::Relaxed) {
        return Err("The application is quitting: no process starts any more.".into());
    }

    // The watcher first: once the process runs, nothing may fail before it
    // has the process, or it would run unwatched.
    let stop = Arc::new(AtomicBool::new(false));
    let (watch_sender, watch_receiver) = mpsc::channel::<Watched>();
    {
        let app = app.clone();
        let watched_alias = alias.clone();
        let stop = Arc::clone(&stop);
        thread::Builder::new()
            .name(format!("process {alias}"))
            .spawn(move || {
                // Disconnected: the process did not start.
                if let Ok(watched) = watch_receiver.recv() {
                    watch(&app, &watched_alias, watched, &stop);
                }
            })
            .map_err(|error| format!("The thread watching \"{alias}\" cannot start: {error}."))?;
    }

    let mut command = processes.command(program, arguments, &cwd, &payload.env);
    let mut child = command
        .spawn()
        .map_err(|error| format!("Cannot start \"{program}\" in {}: {error}.", cwd.display()))?;
    let pid = child.id();

    let readers = Readers::new();
    let deliver = |stream: &'static str| {
        let app = app.clone();
        let alias = alias.clone();
        move |output: &[u8]| send_output(&app, &alias, pid, stream, output)
    };
    let pipes = (child.stdin.take(), child.stdout.take(), child.stderr.take());
    let threads = match pipes {
        (Some(stdin), Some(stdout), Some(stderr)) => writer(&alias, stdin).and_then(|stdin| {
            readers.forward(format!("stdout of {alias}"), stdout, deliver("stdout"))?;
            readers.forward(format!("stderr of {alias}"), stderr, deliver("stderr"))?;
            Ok(stdin)
        }),
        _ => Err(io::Error::other("a pipe is missing")),
    };
    let stdin = match threads {
        Ok(stdin) => stdin,
        Err(error) => {
            kill(&mut child);
            let _ = child.wait();
            return Err(format!("Cannot follow \"{alias}\": {error}.").into());
        }
    };

    running.insert(
        alias,
        Running {
            pid,
            command: payload.command.clone(),
            stdin: Some(stdin),
            stop,
        },
    );
    drop(running);
    // The watcher is alive: it only ends once it has had the process.
    let _ = watch_sender.send(Watched { child, readers });

    Ok(json!({ "pid": pid }))
}

/// Waits for the process to end, ending it when asked to, then tells PHP.
fn watch(app: &AppHandle, alias: &str, mut watched: Watched, stop: &Arc<AtomicBool>) {
    let pid = watched.child.id();
    let status = wait(&mut watched.child, stop);
    // The process is its group: what it left behind ends with it.
    #[cfg(unix)]
    signal_group(&watched.child, libc::SIGTERM);

    let state = app.state::<State>();
    let mut running = lock(&state.child_processes.running);
    // Unless it was started again under the same alias since.
    if running
        .get(alias)
        .is_some_and(|running| Arc::ptr_eq(&running.stop, stop))
    {
        running.remove(alias);
    }
    drop(running);

    // Its last words before its end.
    watched.readers.wait(LAST_WORDS);
    // What did not heed SIGTERM.
    #[cfg(unix)]
    signal_group(&watched.child, libc::SIGKILL);

    if let Err(error) = &status {
        eprintln!("[process] Lost track of \"{alias}\": {error}.");
    }
    let code = status.as_ref().ok().and_then(ExitStatus::code);
    events::send(
        app,
        "childProcess.exited",
        json!({ "alias": alias, "pid": pid, "code": code }),
    );
}

fn wait(child: &mut Child, stop: &AtomicBool) -> io::Result<ExitStatus> {
    let mut asked_at: Option<Instant> = None;

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }

        match asked_at {
            None if stop.load(Ordering::Relaxed) => {
                terminate(child);
                asked_at = Some(Instant::now());
            }
            Some(asked_at) if asked_at.elapsed() > GRACE => kill(child),
            _ => {}
        }

        thread::sleep(POLL);
    }
}

/// Asks the process group to end, as a terminal would: SIGTERM, which a
/// worker handles to finish its job first. Windows has no such thing.
fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if signal_group(child, libc::SIGTERM) {
        return;
    }

    let _ = child.kill();
}

/// Kills the process group, or what is left of it.
fn kill(child: &mut Child) {
    #[cfg(unix)]
    signal_group(child, libc::SIGKILL);

    // Already gone if it fails.
    let _ = child.kill();
}

#[cfg(unix)]
fn signal_group(child: &Child, signal: libc::c_int) -> bool {
    let Ok(pid) = libc::pid_t::try_from(child.id()) else {
        return false;
    };

    // SAFETY: kill() has no memory effect. The group is the one the process
    // leads (process_group(0)), and a group id is not reused while the group
    // has members.
    unsafe { libc::kill(-pid, signal) == 0 }
}

/// The thread that writes to the standard input, for as long as the process
/// runs.
fn writer(alias: &str, mut stdin: ChildStdin) -> io::Result<mpsc::Sender<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();
    let alias = alias.to_owned();
    thread::Builder::new()
        .name(format!("stdin of {alias}"))
        .spawn(move || {
            for data in receiver {
                if let Err(error) = stdin.write_all(&data).and_then(|()| stdin.flush()) {
                    eprintln!("[process] Cannot write to \"{alias}\": {error}.");
                    return;
                }
            }
        })?;

    Ok(sender)
}

/// One event at a time: until PHP handled it, the pipe fills and the
/// process waits to write more, rather than piling events up in front of the
/// pages.
fn send_output(app: &AppHandle, alias: &str, pid: u32, stream: &str, output: &[u8]) {
    let (handled, waiting) = mpsc::sync_channel(1);
    events::send_then(
        app,
        "childProcess.output",
        json!({ "alias": alias, "pid": pid, "stream": stream, "output": String::from_utf8_lossy(output) }),
        move || {
            let _ = handled.send(());
        },
    );
    // Disconnected only if PHP is gone: nothing to wait for then.
    let _ = waiting.recv();
}

fn not_running(alias: &str) -> NativeError {
    not_found(format!("No process named \"{alias}\" is running."))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn processes() -> ChildProcesses {
        ChildProcesses::new(
            PathBuf::from("/projects/notes"),
            vec![
                (
                    "NATIVELLA_DATA_DIR",
                    "/home/me/.local/share/notes".to_owned(),
                ),
                ("NATIVELLA_SECRET", "0123abcd".to_owned()),
            ],
        )
    }

    fn variable<'a>(command: &'a Command, name: &str) -> Option<Option<&'a OsStr>> {
        command
            .get_envs()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    }

    #[test]
    fn processes_get_the_variables_of_nativella() {
        let command = processes().command(
            "php",
            &["bin/console".to_owned()],
            Path::new("/projects/notes"),
            &HashMap::new(),
        );

        assert_eq!(
            variable(&command, "NATIVELLA_DATA_DIR"),
            Some(Some(OsStr::new("/home/me/.local/share/notes")))
        );
        assert_eq!(
            variable(&command, "NATIVELLA_SECRET"),
            Some(Some(OsStr::new("0123abcd")))
        );
    }

    /// What `forward` hands over, then the end, in their order.
    fn record(events: &Arc<Mutex<Vec<String>>>) -> impl Fn(&[u8]) + Send + 'static {
        let events = Arc::clone(events);

        move |output: &[u8]| {
            // PHP takes its time: more than the pipes may stay quiet.
            thread::sleep(Duration::from_millis(300));
            lock(&events).push(String::from_utf8_lossy(output).into_owned());
        }
    }

    #[test]
    fn the_end_comes_after_the_last_output_however_long_php_takes() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let readers = Readers::new();
        readers
            .forward("stdout".to_owned(), &b"done\n"[..], record(&events))
            .unwrap();
        readers
            .forward("stderr".to_owned(), &b"warning\n"[..], record(&events))
            .unwrap();

        readers.wait(Duration::from_millis(100));
        lock(&events).push("exited".to_owned());

        let events = lock(&events);
        assert_eq!(events.len(), 3);
        assert_eq!(events[2], "exited");
    }

    #[test]
    fn a_pipe_left_open_does_not_hold_the_end() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (reader, mut writer) = io::pipe().unwrap();
        writer.write_all(b"last words").unwrap();
        let readers = Readers::new();
        readers
            .forward("stdout".to_owned(), reader, record(&events))
            .unwrap();

        let started = Instant::now();
        readers.wait(Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(*lock(&events), ["last words"]);
        drop(writer);
    }

    #[test]
    fn a_character_cut_in_two_comes_whole() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (reader, mut writer) = io::pipe().unwrap();
        let recorded = Arc::clone(&events);
        let readers = Readers::new();
        readers
            .forward("stdout".to_owned(), reader, move |output: &[u8]| {
                lock(&recorded).push(String::from_utf8_lossy(output).into_owned());
            })
            .unwrap();

        writer
            .write_all("caf\u{e9}".as_bytes().split_last().unwrap().1)
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        writer.write_all(&[0xa9, b'!']).unwrap();
        drop(writer);
        readers.wait(Duration::from_secs(1));

        assert_eq!(lock(&events).concat(), "caf\u{e9}!");
        assert!(
            lock(&events)
                .iter()
                .all(|output| !output.contains('\u{fffd}'))
        );
    }

    #[test]
    fn the_environment_of_the_call_wins() {
        let env = HashMap::from([
            ("NATIVELLA_DATA_DIR".to_owned(), "/srv/notes".to_owned()),
            ("APP_ENV".to_owned(), "prod".to_owned()),
        ]);
        let command = processes().command("php", &[], Path::new("/projects/notes"), &env);

        assert_eq!(
            variable(&command, "NATIVELLA_DATA_DIR"),
            Some(Some(OsStr::new("/srv/notes")))
        );
        assert_eq!(
            variable(&command, "APP_ENV"),
            Some(Some(OsStr::new("prod")))
        );
    }
}
