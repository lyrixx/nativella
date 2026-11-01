//! Android has no terminal: what the shell and PHP write to stdout and stderr
//! goes to logcat instead, under the "nativella" tag.

use std::ffi::{CString, c_char, c_int};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;
use std::thread;

const ANDROID_LOG_INFO: c_int = 4;
const ANDROID_LOG_WARN: c_int = 5;

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

pub(crate) fn redirect_stderr() {
    let mut fds = [0; 2];
    // SAFETY: pipe() fills the two descriptors it is given.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    // SAFETY: both descriptors are open; duplicating onto 1 and 2 replaces
    // the ones Android points to /dev/null.
    unsafe {
        libc::dup2(fds[1], libc::STDOUT_FILENO);
        libc::dup2(fds[1], libc::STDERR_FILENO);
    }
    // SAFETY: the read end belongs to this File alone from now on.
    let mut reader = BufReader::new(unsafe { File::from_raw_fd(fds[0]) });

    thread::Builder::new()
        .name("logcat".to_owned())
        .spawn(move || {
            // Never stop at a line that is not UTF-8 (PHP writes what it is
            // given): the pipe would close, and every later write to stdout
            // or stderr would fail.
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    // Every write end is closed: nothing more can come.
                    Ok(0) => return,
                    Ok(_) => {
                        let text = String::from_utf8_lossy(&line);
                        write(ANDROID_LOG_INFO, text.trim_end_matches(['\n', '\r']));
                    }
                    Err(error) => {
                        write(
                            ANDROID_LOG_WARN,
                            &format!("Stopped reading stdout and stderr: {error}."),
                        );

                        return;
                    }
                }
            }
        })
        .expect("The OS can spawn the logcat thread.");
}

fn write(priority: c_int, text: &str) {
    // A NUL byte would end the line there.
    let text = CString::new(text.replace('\0', "\u{fffd}")).expect("No NUL byte is left.");

    // SAFETY: both strings are valid and NUL-terminated.
    unsafe { __android_log_write(priority, c"nativella".as_ptr(), text.as_ptr()) };
}
