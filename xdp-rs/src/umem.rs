use std::marker::PhantomData;

use libc::SOL_XDP;

use crate::constants;
use crate::error::Error;
use crate::sys::mmap::{Behavior, Protection, Visibility};
use crate::sys::socket::XdpMmapOffsets;
use crate::sys::{self, mmap::MmapRegion, socket::Socket};
use crate::Result;

#[derive(Debug)]
pub struct Umem<'a> {
    socket: Socket,
    producer: RingDriver<'a>,
    consumer: RingDriver<'a>,
    buffer: MmapRegion<'a>,
    config: UmemConfig,
}

#[derive(Debug, Copy, Clone)]
pub struct UmemConfig {
    fill_size: u32,
    comp_size: u32,
    frame_size: u32,
    frame_headroom: u32,
}

impl<'a> Umem<'a> {
    pub fn create(mut socket: Socket, buffer: MmapRegion<'a>) -> Result<Umem<'a>> {
        if buffer.len == 0 {
            return Err(Error::Efault("buffer has no length"));
        }

        if !sys::is_page_aligned(buffer.addr.as_ptr() as *const _) {
            return Err(Error::Efault("buffer is not page aligned"));
        }

        let config = UmemConfig {
            fill_size: constants::DEFAULT_PROD_NUM_DESCS,
            comp_size: constants::DEFAULT_CONS_NUM_DESCS,
            frame_size: constants::FRAME_SIZE as u32,
            frame_headroom: constants::DEFAULT_FRAME_HEADROOM,
        };

        println!("Setting XDP_UMEM_REG");
        socket.set_opt(
            SOL_XDP,
            xdp_sys::XDP_UMEM_REG,
            &xdp_sys::xdp_umem_reg {
                addr: buffer.addr.as_ptr() as u64,
                len: buffer.len as u64,
                chunk_size: config.frame_size,
                headroom: config.frame_headroom,
                flags: 0,
            },
        )?;

        println!("Setting XDP_UMEM_FILL_RING");
        socket.set_opt(SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &config.fill_size)?;

        println!("Setting XDP_UMEM_COMPLETION_RING");
        socket.set_opt(
            SOL_XDP,
            xdp_sys::XDP_UMEM_COMPLETION_RING,
            &config.comp_size,
        )?;

        let offsets = socket.get_opt::<XdpMmapOffsets>()?;

        let fill_ring_len =
            offsets.fr.desc + config.fill_size as u64 * std::mem::size_of::<u64>() as u64;
        let fill_ring_mmap = sys::mmap::builder()
            .fd(socket.fd)
            .visibility(Visibility::Shared)
            .length(fill_ring_len as usize)
            .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

        let comp_ring_len =
            offsets.cr.desc + config.comp_size as u64 * std::mem::size_of::<u64>() as u64;
        let comp_ring_mmap = sys::mmap::builder()
            .fd(socket.fd)
            .visibility(Visibility::Shared)
            .length(comp_ring_len as usize)
            .offset(xdp_sys::XDP_UMEM_PGOFF_COMPLETION_RING as i64)
            .behaviour(Behavior::PopulatePageTables)
            .protection(Protection::Read | Protection::Write)
            .build()?;

        let producer = RingDriver {
            cached_prod: 0,
            cached_cons: config.fill_size,
            mask: config.fill_size - 1,
            size: config.fill_size,
            producer: (fill_ring_mmap.addr.as_ptr() as u64 + offsets.fr.producer) as *mut _,
            consumer: (fill_ring_mmap.addr.as_ptr() as u64 + offsets.fr.consumer) as *mut _,
            ring: (fill_ring_mmap.addr.as_ptr() as u64 + offsets.fr.desc) as *mut _,
            umem_ref: PhantomData,
        };

        let consumer = RingDriver {
            cached_prod: 0,
            cached_cons: config.fill_size,
            mask: config.comp_size - 1,
            size: config.comp_size,
            producer: (comp_ring_mmap.addr.as_ptr() as u64 + offsets.cr.producer) as *mut _,
            consumer: (comp_ring_mmap.addr.as_ptr() as u64 + offsets.cr.consumer) as *mut _,
            ring: (comp_ring_mmap.addr.as_ptr() as u64 + offsets.cr.desc) as *mut _,
            umem_ref: PhantomData,
        };

        Ok(Umem {
            socket,
            producer,
            consumer,
            buffer,
            config,
        })
    }
}

#[derive(Debug)]
pub struct RingDriver<'a> {
    cached_prod: u32,
    cached_cons: u32,
    mask: u32,
    size: u32,
    producer: *mut u32,
    consumer: *mut u32,
    ring: *mut u8,
    // Manually tie the producer's lifetime to the umem buffer
    umem_ref: PhantomData<&'a [u8]>,
}
