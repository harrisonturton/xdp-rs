use std::mem::size_of;

use crate::error::Error;
use crate::ring::{CompRing, FillRing, RingBuffer};
use crate::sys::mmap::{Behavior, Protection, Visibility};
use crate::sys::ptr_offset;
use crate::sys::socket::{Socket, XdpMmapOffsets};
use crate::sys::{self, mmap::Mmap};
use crate::Result;

#[derive(Debug)]
pub struct Umem {
    pub frame_buffer: Mmap,
    pub frame_count: u32,
    pub frame_size: u32,
    pub frame_headroom: u32,
    pub fill: FillRing,
    pub comp: CompRing,
}

impl Umem {
    #[must_use]
    pub fn builder<'a>() -> UmemBuilder<'a> {
        UmemBuilder::default()
    }

    pub fn create(
        sock: &Socket,
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

        sock.set_opt::<xdp_sys::xdp_umem_reg>(
            libc::SOL_XDP,
            xdp_sys::XDP_UMEM_REG,
            &xdp_sys::xdp_umem_reg {
                addr: frame_buffer.addr.as_ptr().addr() as u64,
                len: frame_buffer.len as u64,
                chunk_size: frame_size,
                headroom: frame_headroom,
                flags: 0,
            },
        )?;

        let offsets = sock.get_opt::<XdpMmapOffsets>()?;
        let fill = register_fill_ring(sock, frame_count as usize, &offsets.fr)?;
        let comp = register_completion_ring(sock, frame_count as usize, &offsets.cr)?;

        Ok(Umem {
            frame_buffer,
            frame_count: frame_count as u32,
            frame_size: frame_size as u32,
            frame_headroom: frame_headroom as u32,
            fill,
            comp,
        })
    }

    #[must_use]
    pub fn rings(&mut self) -> (&mut FillRing, &mut CompRing) {
        (&mut self.fill, &mut self.comp)
    }
}

#[must_use]
pub fn register_fill_ring<'a>(
    socket: &Socket,
    frame_count: usize,
    offsets: &xdp_sys::xdp_ring_offset,
) -> Result<FillRing> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &frame_count)?;

    let len = (offsets.desc + frame_count as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = ptr_offset(mmap.addr, offsets.producer as usize);
    let consumer = ptr_offset(mmap.addr, offsets.consumer as usize);
    let descs = ptr_offset(mmap.addr, offsets.desc as usize);

    Ok(RingBuffer::new(frame_count, producer, consumer, descs))
}

#[must_use]
fn register_completion_ring<'a>(
    socket: &Socket,
    frame_count: usize,
    offsets: &xdp_sys::xdp_ring_offset,
) -> Result<CompRing> {
    socket.set_opt(
        libc::SOL_XDP,
        xdp_sys::XDP_UMEM_COMPLETION_RING,
        &frame_count,
    )?;

    let len = (offsets.desc + frame_count as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_COMPLETION_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = ptr_offset(mmap.addr, offsets.producer as usize);
    let consumer = ptr_offset(mmap.addr, offsets.consumer as usize);
    let descs = ptr_offset(mmap.addr, offsets.desc as usize);

    Ok(RingBuffer::new(frame_count, producer, consumer, descs))
}

#[derive(Debug, Default, Clone)]
pub struct UmemBuilder<'a> {
    sock: Option<&'a Socket>,
    frame_count: Option<u32>,
    frame_size: Option<u32>,
    frame_headroom: Option<u32>,
}

impl<'a> UmemBuilder<'a> {
    #[must_use]
    pub fn socket(mut self, sock: &'a Socket) -> Self {
        self.sock = Some(sock);
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
        let sock = self
            .sock
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
        Umem::create(sock, frame_count, frame_size, frame_headroom)
    }
}
