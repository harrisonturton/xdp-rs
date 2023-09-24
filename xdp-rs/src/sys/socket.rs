//! Safe wrapper for [libc::socket] methods.
//!
//! ```
//! let sock = socket::create(AF_XDP, SOCK_RAW, 0)?;
//! sock.set_opt(SOL_XDP, XDP_UMEM_REG, &xdp_umem_reg { ... })?;
//! ```
use crate::error::Error;
use crate::sys::errno;
use crate::Result;

#[must_use]
pub fn create(domain: i32, typ: i32, protocol: i32) -> Result<Socket> {
    Socket::create(domain, typ, protocol)
}

/// Wrapper for a Linux socket file descriptor that provides safe alternatives
/// to the libc socket methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Socket {
    pub fd: i32,
}

impl Socket {
    #[must_use]
    pub fn create(domain: i32, typ: i32, protocol: i32) -> Result<Socket> {
        unsafe {
            match libc::socket(domain, typ, protocol) {
                ret if ret < 0 => Err(Error::Socket(errno())),
                fd => Ok(Socket { fd }),
            }
        }
    }

    #[must_use]
    pub fn set_opt<T>(&mut self, level: i32, opt_name: u32, opt_value: &T) -> Result<()> {
        unsafe {
            match libc::setsockopt(
                self.fd as i32,
                level,
                opt_name as i32,
                super::ptr(opt_value),
                std::mem::size_of::<T>() as u32,
            ) {
                ret if ret < 0 => Err(Error::SetSockOpt(super::errno())),
                _ => Ok(()),
            }
        }
    }

    /// The option to get is indicated by the zero-sized generic [GetSockOpt] type.
    #[must_use]
    pub fn get_opt<O: GetSockOpt>(&mut self) -> Result<O::Value> {
        O::try_get(self)
    }
}

pub trait GetSockOpt {
    type Value;

    fn try_get(socket: &Socket) -> Result<Self::Value>;
}

pub struct XdpMmapOffsets;

impl GetSockOpt for XdpMmapOffsets {
    type Value = xdp_sys::xdp_mmap_offsets;

    fn try_get(socket: &Socket) -> Result<Self::Value> {
        let mut len = std::mem::size_of::<xdp_sys::xdp_mmap_offsets>() as u32;
        let mut buf = Vec::with_capacity(len as usize);

        unsafe {
            getsockopt(
                socket.fd,
                libc::SOL_RAW,
                xdp_sys::XDP_MMAP_OFFSETS as i32,
                buf.as_mut_ptr(),
                &mut len,
            )?;
        };

        if len < std::mem::size_of::<xdp_sys::xdp_mmap_offsets>() as u32 {
            return Err(Error::Efault("returned a byte buffer that is too small"));
        }

        Ok(unsafe { std::ptr::read(buf.as_ptr() as *const _) })
    }
}

// Small utility to avoid repeating the same error handling when using
// [libc::getsockopt] in different implementation of [GetSockOpt].
unsafe fn getsockopt(
    sockfd: i32,
    level: i32,
    optname: i32,
    optval: *mut libc::c_void,
    optlen: *mut libc::socklen_t,
) -> Result<()> {
    if unsafe { libc::getsockopt(sockfd, level, optname, optval, optlen) } < 0 {
        Err(Error::GetSockOpt(errno()))
    } else {
        Ok(())
    }
}
