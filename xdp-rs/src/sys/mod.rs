//! This module contains types-afe and memory-safe wrappers over linux syscalls.
//! Unsafe blocks should be confined to this namespace, and care should be taken
//! to make sure that all the exposed interfaces are memory safe.
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
pub(crate) fn is_page_aligned(mem: *const libc::c_void) -> bool {
    mem as u64 & (libc::_SC_PAGE_SIZE as u64 - 1) == 0
}

#[must_use]
pub fn strerror(code: i32) -> String {
    let msg_ptr = unsafe { libc::strerror(code) };
    let msg_cstr = unsafe { std::ffi::CStr::from_ptr(msg_ptr) };
    msg_cstr.to_string_lossy().to_string()
}
