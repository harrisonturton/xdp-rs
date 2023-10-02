use crate::error::Error;
use crate::sys::mmap::{Behavior, Protection, Visibility};
use crate::sys::{self, mmap::Mmap, socket::Socket};
use crate::Result;
use libc::SOL_XDP;

#[derive(Debug)]
pub struct Umem {
    pub frame_buffer: Mmap,
    pub frame_count: u32,
    pub frame_size: u32,
    pub frame_headroom: u32,
}

impl Umem {
    #[must_use]
    pub fn builder<'a>() -> UmemBuilder<'a> {
        UmemBuilder::default()
    }

    pub fn create<'a>(
        socket: &'a Socket,
        frame_count: u32,
        frame_size: u32,
        frame_headroom: u32,
    ) -> Result<Umem> {
        if frame_count == 0 {
            return Err(Error::InvalidArgument("frame buffer cannot be zero length"));
        }

        let frame_buffer = Mmap::builder()
            .length((frame_count * frame_size) as usize)
            .visibility(Visibility::Private)
            .behaviour(Behavior::Anonymous)
            .protection(Protection::Read | Protection::Write | Protection::Exec)
            .build()?;

        if !sys::is_page_aligned(frame_buffer.addr.as_ptr()) {
            return Err(Error::Efault("buffer is not page aligned"));
        }

        let reg = xdp_sys::xdp_umem_reg {
            addr: frame_buffer.addr.as_ptr().addr() as u64,
            len: frame_buffer.len as u64,
            chunk_size: frame_size,
            headroom: frame_headroom,
            flags: 0,
        };
        socket.set_opt::<xdp_sys::xdp_umem_reg>(SOL_XDP, xdp_sys::XDP_UMEM_REG, &reg)?;

        Ok(Umem {
            frame_buffer,
            frame_count: frame_count as u32,
            frame_size: frame_size as u32,
            frame_headroom: frame_headroom as u32,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct UmemBuilder<'a> {
    socket: Option<&'a Socket>,
    frame_count: Option<u32>,
    frame_size: Option<u32>,
    frame_headroom: Option<u32>,
}

impl<'a> UmemBuilder<'a> {
    #[must_use]
    pub fn socket(mut self, socket: &'a Socket) -> Self {
        self.socket = Some(socket);
        self
    }

    #[must_use]
    pub fn frame_count(mut self, frame_count: u32) -> Self {
        self.frame_count = Some(frame_count);
        self
    }

    #[must_use]
    pub fn frame_size(mut self, frame_size: u32) -> Self {
        self.frame_size = Some(frame_size);
        self
    }

    #[must_use]
    pub fn frame_headroom(mut self, frame_headroom: u32) -> Self {
        self.frame_headroom = Some(frame_headroom);
        self
    }

    pub fn build(self) -> Result<Umem> {
        let socket = self
            .socket
            .ok_or_else(|| Error::InvalidArgument("socket must be specified"))?;
        let frame_count = self
            .frame_count
            .ok_or_else(|| Error::InvalidArgument("frame_count must be specified"))?;
        let frame_size = self
            .frame_size
            .ok_or_else(|| Error::InvalidArgument("frame_size must be specified"))?;
        let frame_headroom = self
            .frame_headroom
            .ok_or_else(|| Error::InvalidArgument("frame_headroom must be specified"))?;
        Umem::create(socket, frame_count, frame_size, frame_headroom)
    }
}
