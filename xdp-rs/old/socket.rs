use crate::error::{self, Error};
use crate::util::{ptr, sizeof};
use crate::Result;
use libc::{setsockopt, socket, AF_XDP, SOCK_RAW, SOL_XDP};
use xdp_sys::{xdp_umem_reg, XDP_UMEM_REG};

#[derive(Debug)]
struct XdpSock {
    fd: i32,
}

impl XdpSock {
    pub(crate) fn new(fd: i32) -> Self {
        XdpSock { fd }
    }

    pub unsafe fn try_create() -> Result<Self> {
        let sock = socket(AF_XDP, SOCK_RAW, 0);
        error::check(sock).or_err(Error::CreateSocket("socket() failed"))?;

        let reg = xdp_umem_reg {
            addr: 0,
            len: 0,
            chunk_size: 0,
            headroom: 0,
            flags: 0,
        };

        let fd = setsockopt(sock, SOL_XDP, XDP_UMEM_REG as i32, ptr(reg), sizeof(reg));
        error::check(sock).or_err(Error::CreateSocket("setsockopt failed"))?;

        Ok(XdpSock::new(fd))
    }
}
