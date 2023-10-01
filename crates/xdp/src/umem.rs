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

#[derive(Debug)]
pub struct UmemConfig<'a> {
    pub socket: &'a Socket,
    pub frame_count: u32,
    pub frame_size: u32,
    pub frame_headroom: u32,
}

impl Umem {
    pub fn create<'a>(config: UmemConfig<'a>) -> Result<Umem> {
        if config.frame_count == 0 {
            return Err(Error::InvalidArgument("frame buffer cannot be zero length"));
        }

        println!("Umem len: {}", config.frame_count * config.frame_size);

        let frame_buffer = sys::mmap::builder()
            .length((config.frame_count * config.frame_size) as usize)
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
            chunk_size: config.frame_size,
            headroom: config.frame_headroom,
            flags: 0,
        };
        config
            .socket
            .set_opt::<xdp_sys::xdp_umem_reg>(SOL_XDP, xdp_sys::XDP_UMEM_REG, &reg)?;

        Ok(Umem {
            frame_buffer,
            frame_count: config.frame_count as u32,
            frame_size: config.frame_size as u32,
            frame_headroom: config.frame_headroom as u32,
        })
    }
}
