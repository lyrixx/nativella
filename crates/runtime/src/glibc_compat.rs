//! A static PHP built against older glibc headers calls the resolver through
//! its old names (`__res_nsearch`…, for `dns_get_record()` and friends), which
//! glibc 2.34 and later only keeps as compatibility symbols, out of reach of
//! the linker. These forward to the current names.
//!
//! Only for Linux builds that link PHP statically (see build.rs).

use std::ffi::{c_char, c_int, c_uchar, c_void};

unsafe extern "C" {
    fn res_nsearch(
        state: *mut c_void,
        name: *const c_char,
        class: c_int,
        kind: c_int,
        answer: *mut c_uchar,
        length: c_int,
    ) -> c_int;
    fn dn_expand(
        message: *const c_uchar,
        end: *const c_uchar,
        compressed: *const c_uchar,
        expanded: *mut c_char,
        length: c_int,
    ) -> c_int;
    fn dn_skipname(compressed: *const c_uchar, end: *const c_uchar) -> c_int;
}

/// # Safety
///
/// As `res_nsearch(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __res_nsearch(
    state: *mut c_void,
    name: *const c_char,
    class: c_int,
    kind: c_int,
    answer: *mut c_uchar,
    length: c_int,
) -> c_int {
    unsafe { res_nsearch(state, name, class, kind, answer, length) }
}

/// # Safety
///
/// As `dn_expand(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dn_expand(
    message: *const c_uchar,
    end: *const c_uchar,
    compressed: *const c_uchar,
    expanded: *mut c_char,
    length: c_int,
) -> c_int {
    unsafe { dn_expand(message, end, compressed, expanded, length) }
}

/// # Safety
///
/// As `dn_skipname(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __dn_skipname(compressed: *const c_uchar, end: *const c_uchar) -> c_int {
    unsafe { dn_skipname(compressed, end) }
}
