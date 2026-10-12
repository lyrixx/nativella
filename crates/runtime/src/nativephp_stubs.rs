//! The mobile PHP builds of mobile/php/fetch.sh come from NativePHP and carry
//! its `nativephp` extension, which calls back into the host through these
//! functions. Nativella has its own bridge: they answer that nothing is
//! available, should PHP code call `nativephp_*()` functions.
//!
//! Signatures from NativePHP's BridgeRouter.swift (MIT).

#![allow(non_snake_case)]

use std::ffi::{c_char, c_void};
use std::ptr;

#[unsafe(no_mangle)]
pub extern "C" fn NativePHPCan(_function: *const c_char) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn NativePHPCall(
    _function: *const c_char,
    _parameters: *const c_char,
) -> *const c_char {
    ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn NativeUI_RegisterRegion(_region: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn NativeUI_UnregisterRegion() {}

#[unsafe(no_mangle)]
pub extern "C" fn NativeElement_RegisterRegion(_region: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn NativeElement_UnregisterRegion() {}

#[unsafe(no_mangle)]
pub extern "C" fn NativeElement_PostTreeUpdate() {}
