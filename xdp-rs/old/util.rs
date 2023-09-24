use libc::c_void;

#[must_use]
pub fn ptr<T>(val: T) -> *const c_void {
    std::ptr::addr_of!(val) as *const _
}

#[must_use]
pub fn sizeof<T>(_val: T) -> u32 {
    std::mem::size_of::<T>() as u32
}
