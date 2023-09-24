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
use libc::MAP_FAILED;

use super::errno;
use crate::{error::Error, Result};
use std::marker::PhantomData;

/// Begin configuring a private mmap.
#[must_use]
pub fn private() -> MmapBuilder {
    MmapBuilder::new(libc::MAP_PRIVATE)
}

/// Begin configuring a shared mmap.
#[must_use]
pub fn shared() -> MmapBuilder {
    MmapBuilder::new(libc::MAP_SHARED)
}

/// Represents a region of mmapped memory. The lifetime refers to the region of
/// memory. `munmap` will be called automatically when this value is dropped.
#[derive(Debug)]
pub struct MmapRegion<'a> {
    addr: *const libc::c_void,
    length: usize,
    phantom: PhantomData<&'a [u8]>,
}

impl<'a> MmapRegion<'a> {
    pub fn munmap(&mut self) -> Result<()> {
        match unsafe { libc::munmap(self.addr as *mut _, self.length) } {
            ret if ret < 0 => Err(Error::Munmap(errno())),
            _ => Ok(()),
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub unsafe fn addr(&self) -> *const libc::c_void {
        self.addr
    }
}

impl<'a> Drop for MmapRegion<'a> {
    fn drop(&mut self) {
        self.munmap().expect("failed to munmap");
    }
}

/// Used to configure and create an instance of mmapped memory.
#[derive(Default)]
pub struct MmapBuilder {
    length: usize,
    prot: i32,
    flags: i32,
    fd: Option<i32>,
    offset: i64,
}

impl MmapBuilder {
    #[must_use]
    pub(crate) fn new(visibility: i32) -> Self {
        MmapBuilder {
            flags: visibility,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn length(mut self, length: usize) -> Self {
        self.length = length;
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
            self.length,
            self.prot,
            self.flags,
            self.fd.unwrap_or(-1),
            self.offset
        );

        let buf = unsafe {
            match libc::mmap(
                std::ptr::null_mut(),
                self.length,
                self.prot,
                self.flags,
                self.fd.unwrap_or(-1),
                self.offset as i64,
            ) {
                MAP_FAILED => Err(Error::Mmap(errno())),
                buf => Ok(buf),
            }
        }?;

        Ok(MmapRegion {
            addr: buf,
            length: self.length,
            phantom: PhantomData,
        })
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
