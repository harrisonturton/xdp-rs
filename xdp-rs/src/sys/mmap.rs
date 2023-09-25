//! Safe wrapper for [libc::mmap].
//!
//! ```
//! let mem = mmap::private()
//!     .length(100)
//!     .behaviour(Behavior::Anonymous)
//!     .protection(Protection::Read | Protection::Write)
//!     .build()?;
//! ```
//!
//! Automatically calls [libc::munmap] when the value is dropped.
use super::errno;
use crate::{error::Error, Result};
use libc::MAP_FAILED;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Represents a region of mmapped memory. The lifetime refers to the region of
/// memory. `munmap` will be called automatically when this value is dropped.
#[derive(Debug)]
pub struct MmapRegion<'a> {
    pub addr: NonNull<u8>,
    pub len: usize,
    _marker: PhantomData<&'a u8>,
}

impl<'a> Drop for MmapRegion<'a> {
    fn drop(&mut self) {
        munmap(self).expect("failed to munmap");
    }
}

pub fn mmap<'a>(
    addr: Option<NonNull<u8>>,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> Result<MmapRegion<'a>> {
    let ptr = addr.map(NonNull::as_ptr).unwrap_or(std::ptr::null_mut());
    let ret = unsafe { libc::mmap(ptr as *mut _, len, prot, flags, fd, offset) };

    if ret == MAP_FAILED {
        return Err(Error::Mmap(errno()));
    }

    Ok(MmapRegion {
        addr: NonNull::new(ret as *mut _).ok_or(Error::Efault("mmap returned null pointer"))?,
        len,
        _marker: PhantomData,
    })
}

#[must_use]
pub fn munmap<'a>(region: &MmapRegion<'a>) -> Result<()> {
    let ret = unsafe { libc::munmap(region.addr.as_ptr() as *mut _, region.len) };

    if ret == -1 {
        return Err(Error::Munmap(errno()));
    }

    return Ok(());
}

/// Begin configuring a mmap.
#[must_use]
pub fn builder() -> MmapBuilder {
    MmapBuilder::default()
}

/// Used to configure and create an instance of mmapped memory.
#[derive(Default)]
pub struct MmapBuilder {
    len: usize,
    prot: i32,
    flags: i32,
    fd: Option<i32>,
    offset: i64,
    visibility: Option<i32>,
    addr: Option<NonNull<u8>>,
}

impl MmapBuilder {
    #[must_use]
    pub fn length(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    #[must_use]
    pub fn visibility<I: Into<i32>>(mut self, visibility: I) -> Self {
        self.visibility = Some(visibility.into());
        self
    }

    #[must_use]
    pub fn addr(mut self, addr: Option<NonNull<u8>>) -> Self {
        self.addr = addr;
        self
    }

    #[must_use]
    pub fn fd(mut self, fd: i32) -> Self {
        self.fd = Some(fd);
        self
    }

    #[must_use]
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    #[must_use]
    pub fn protection<I: Into<i32>>(mut self, protection: I) -> Self {
        self.prot |= protection.into();
        self
    }

    #[must_use]
    pub fn behaviour<I: Into<i32>>(mut self, behavior: I) -> Self {
        self.flags |= behavior.into();
        self
    }

    #[must_use]
    pub fn build<'a>(self) -> Result<MmapRegion<'a>> {
        println!(
            "{:?} {:?} {:?} {:?} {:?}",
            self.len,
            self.prot,
            self.flags,
            self.fd.unwrap_or(-1),
            self.offset
        );

        let visibility = self
            .visibility
            .ok_or(Error::Efault("must specify mmap visibility"))?;

        mmap(
            self.addr,
            self.len,
            self.prot,
            self.flags | visibility,
            self.fd.unwrap_or(-1),
            self.offset as i64,
        )
    }
}

#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Visibility {
    Private,
    Shared,
}

impl From<Visibility> for i32 {
    fn from(value: Visibility) -> Self {
        match value {
            Visibility::Private => libc::MAP_PRIVATE,
            Visibility::Shared => libc::MAP_SHARED,
        }
    }
}

/// Configures the behaviour of the mmap. Roughly equivalent to the different
/// flag values in the syscall.
#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Behavior {
    Anonymous,
    PopulatePageTables,
}

impl From<Behavior> for i32 {
    fn from(value: Behavior) -> Self {
        match value {
            Behavior::Anonymous => libc::MAP_ANONYMOUS,
            Behavior::PopulatePageTables => libc::MAP_POPULATE,
        }
    }
}

/// Memory protection flags for mmap.
#[derive(Hash, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Protection {
    Read,
    Write,
    Exec,
}

impl std::ops::BitOr<Behavior> for Behavior {
    type Output = i32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as i32 | rhs as i32
    }
}

impl std::ops::BitOr<Behavior> for i32 {
    type Output = i32;

    fn bitor(self, rhs: Behavior) -> Self::Output {
        self as i32 | rhs as i32
    }
}

impl From<Protection> for i32 {
    fn from(value: Protection) -> Self {
        match value {
            Protection::Read => libc::PROT_READ,
            Protection::Write => libc::PROT_WRITE,
            Protection::Exec => libc::PROT_EXEC,
        }
    }
}

impl std::ops::BitOr<Protection> for Protection {
    type Output = i32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self as i32 | rhs as i32
    }
}

impl std::ops::BitOr<Protection> for i32 {
    type Output = i32;

    fn bitor(self, rhs: Protection) -> Self::Output {
        self as i32 | rhs as i32
    }
}
