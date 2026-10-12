//! Android has no terminal: what the shell and PHP write to stdout and stderr
//! goes to logcat instead, under the "nativella" tag.

use std::ffi::{CString, c_char, c_int};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;
use std::thread;

const ANDROID_LOG_INFO: c_int = 4;

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
    let reader = BufReader::new(unsafe { File::from_raw_fd(fds[0]) });

    thread::Builder::new()
        .name("logcat".to_owned())
        .spawn(move || {
            let tag = CString::new("nativella").expect("The tag has no NUL byte.");
            for line in reader.lines().map_while(Result::ok) {
                let Ok(text) = CString::new(line) else {
                    continue;
                };
                // SAFETY: both strings are valid and NUL-terminated.
                unsafe { __android_log_write(ANDROID_LOG_INFO, tag.as_ptr(), text.as_ptr()) };
            }
        })
        .expect("The OS can spawn the logcat thread.");
}
