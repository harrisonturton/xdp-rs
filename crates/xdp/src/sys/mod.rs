//! This module contains types-afe and memory-safe wrappers over linux syscalls.
//! Unsafe blocks should be confined to this namespace, and care should be taken
//! to make sure that all the exposed interfaces are memory safe.

use std::{ffi::CString, ptr::NonNull};

use crate::{error::Error, Result};
pub mod mmap;
pub mod socket;

#[must_use]
pub(crate) fn ptr<T>(val: T) -> *const libc::c_void {
    std::ptr::addr_of!(val) as *const _
}

#[must_use]
pub(crate) fn mut_ptr<T>(val: T) -> *mut libc::c_void {
    std::ptr::addr_of!(val) as *mut _
}

#[must_use]
pub(crate) fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

#[must_use]
pub(crate) fn is_page_aligned<T>(mem: *const T) -> bool {
    mem as u64 & (libc::_SC_PAGE_SIZE as u64 - 1) == 0
}

#[must_use]
pub fn strerror(code: i32) -> String {
    let msg_ptr = unsafe { libc::strerror(code) };
    let msg_cstr = unsafe { std::ffi::CStr::from_ptr(msg_ptr) };
    msg_cstr.to_string_lossy().to_string()
}

#[must_use]
pub fn ptr_offset<T, S>(addr: NonNull<T>, offset: usize) -> *mut S {
    (usize::from(addr.addr()) + offset) as *mut S
}

#[must_use]
pub fn if_nametoindex(name: String) -> Result<u32> {
    let ret = unsafe {
        let cstr = CString::new(name).map_err(|_| Error::Efault("bad ifindex name"))?;
        libc::if_nametoindex(cstr.as_ptr())
    };

    if ret == 0 {
        Err(Error::IfNameToIndex(errno()))
    } else {
        Ok(ret)
    }
}

pub fn poll(fd: u32, events: i16) -> i32 {
    let mut pollfd = libc::pollfd {
        fd: fd as i32,
        events,
        revents: 0,
    };
    unsafe { libc::poll(&mut pollfd, 1, -1) }
}
