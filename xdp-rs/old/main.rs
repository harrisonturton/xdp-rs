use libc::{
    AF_XDP, MAP_ANONYMOUS, MAP_POPULATE, MAP_PRIVATE, MAP_SHARED, PROT_READ, PROT_WRITE, SOCK_RAW,
    SOL_XDP,
};
use std::ptr;

use crate::raw::{ptr, sizeof};

pub const NUM_FRAMES: u32 = 4096;
pub const PACKET_BUFFER_SIZE: usize = (NUM_FRAMES * UMEM_DEFAULT_FRAME_SIZE) as usize;
pub const RX_BATCH_SIZE: u32 = 64;

pub const CONS_RING_DEFAULT_NUM_DESCS: u32 = 2048;
pub const PROD_RING_DEFAULT_NUM_DESCS: u32 = 2048;

pub const UMEM_DEFUALT_FRAME_HEADROOM: u32 = 0;
pub const UMEM_DEFAULT_FRAME_SIZE: u32 = 2048;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to create socket: {0}")]
    Socket(i32),
    #[error("failed to mmap: {0}")]
    Mmap(i32),
    #[error("failed to setsockopt: {0}")]
    SetSockOpt(i32),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
    #[error("EFAULT: {0}")]
    Efault(&'static str),
}

/// list for packets with xdp
#[derive(argh::FromArgs)]
struct Args {
    /// network device name
    #[argh(option)]
    ifname: String,
    /// index of the netdev rx/tx queue
    #[argh(option)]
    queue_id: u32,
}

fn main() -> Result<()> {
    let Args { ifname, queue_id } = argh::from_env::<Args>();

    let packet_buffer = unsafe {
        raw::mmap(
            ptr::null_mut(),
            PACKET_BUFFER_SIZE,
            PROT_READ | PROT_WRITE,
            MAP_ANONYMOUS | MAP_PRIVATE,
            -1,
            0,
        )
    }?;

    let umem = configure_umem(packet_buffer, PACKET_BUFFER_SIZE)?;

    // xsk_umem__create(
    //   struct xsk_umem **umem_ptr,
    //   void *umem_area,
    //   __u64 size,
    //   struct xsk_ring_prod *fill,
    //   struct xsk_ring_cons *comp,
    //   const struct xsk_umem_config *usr_config);

    //   ret = xsk_umem__create(
    //       &umem->umem, buffer, size, &umem->fill, &umem->comp, NULL);

    Ok(())
}

fn configure_umem(buffer: *mut libc::c_void, size: usize) -> Result<UmemInfo> {
    let offsets: xdp_sys::xdp_mmap_offsets = todo!();
    let umem: Umem = todo!();

    if buffer.is_null() {
        return Err(Error::InvalidArgument("buffer is null"))?;
    }

    if size != 0 && !raw::is_page_aligned(buffer) {
        return Err(Error::InvalidArgument("buffer is not page aligned"))?;
    }

    let sock = unsafe { raw::socket(AF_XDP, SOCK_RAW, 0) }?;

    let cfg = UmemConfig {
        fill_size: PROD_RING_DEFAULT_NUM_DESCS,
        comp_size: CONS_RING_DEFAULT_NUM_DESCS,
        frame_size: UMEM_DEFAULT_FRAME_SIZE,
        frame_headroom: UMEM_DEFUALT_FRAME_HEADROOM,
    };

    let reg = xdp_sys::xdp_umem_reg {
        addr: buffer as u64,
        len: size as u64,
        chunk_size: cfg.frame_size,
        headroom: cfg.frame_headroom,
        flags: 0,
    };

    raw::setsockopt(
        sock,
        SOL_XDP,
        xdp_sys::XDP_UMEM_REG as i32,
        ptr(reg),
        sizeof(reg),
    )?;

    unsafe {
        raw::setsockopt(
            sock,
            SOL_XDP,
            xdp_sys::XDP_UMEM_FILL_RING as i32,
            ptr(cfg.fill_size),
            sizeof(cfg.fill_size),
        )
    }?;

    unsafe {
        raw::setsockopt(
            sock,
            SOL_XDP,
            xdp_sys::XDP_UMEM_COMPLETION_RING as i32,
            ptr(cfg.comp_size),
            sizeof(cfg.comp_size),
        )
    }?;

    unsafe {
        raw::setsockopt(
            sock,
            SOL_XDP,
            xdp_sys::XDP_MMAP_OFFSETS as i32,
            ptr(offsets),
            sizeof(offsets),
        )
    }?;

    let fill_map = unsafe {
        raw::mmap(
            ptr::null_mut(),
            (offsets.fr.desc + cfg.fill_size as u64 * std::mem::size_of::<u64>() as u64) as usize,
            PROT_READ | PROT_WRITE,
            // TODO: What does MAP_POPULATE do here?
            MAP_SHARED | MAP_POPULATE,
            umem.fd,
            xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64,
        )
    }?;

    let fill = RingProducer {
        cached_prod: 0,
        cached_cons: cfg.fill_size,
        size: cfg.fill_size,
        mask: cfg.fill_size - 1,
        producer: (fill_map as u64 + offsets.fr.producer as u64) as *mut _,
        consumer: (fill_map as u64 + offsets.fr.consumer as u64) as *mut _,
        ring: (fill_map as u64 + offsets.fr.desc) as *mut _,
        flags: ptr::null_mut(),
    };

    let comp_map = unsafe {
        raw::mmap(
            ptr::null_mut(),
            (offsets.cr.desc + cfg.comp_size as u64 * std::mem::size_of::<u64>() as u64) as usize,
            PROT_READ | PROT_WRITE,
            // TODO: What does MAP_POPULATE do here?
            MAP_SHARED | MAP_POPULATE,
            umem.fd,
            xdp_sys::XDP_UMEM_PGOFF_COMPLETION_RING as i64,
        )
    }?;

    let comp = RingConsumer {
        // cached_prod: 0,
        // cached_cons: cfg.fill_size,
        // size: cfg.fill_size,
        // mask: cfg.fill_size - 1,
        // producer: (fill_map as u64 + offsets.fr.producer as u64) as *mut _,
        // consumer: (fill_map as u64 + offsets.fr.consumer as u64) as *mut _,
        // ring: (fill_map as u64 + offsets.fr.desc) as *mut _,
        // flags: ptr::null_mut(),
    };

    // mmap fill ring
    // mmap completion ring
    // initialize umem and return

    todo!()
}

#[derive(Debug)]
struct RingProducer {
    cached_prod: u32,
    cached_cons: u32,
    mask: u32,
    size: u32,
    producer: *mut u32,
    consumer: *mut u32,
    ring: *mut libc::c_void,
    flags: *mut u32,
}

#[derive(Debug)]
struct RingConsumer {
    cached_prod: u32,
    cached_cons: u32,
    mask: u32,
    size: u32,
}

// xsk_umem_info
struct UmemInfo {
    buffer: *mut libc::c_void,
    umem_size: u64,
}

struct UmemConfig {
    fill_size: u32,
    comp_size: u32,
    frame_size: u32,
    frame_headroom: u32,
}

// xsk_umem
// https://github.com/digitalocean/linux-coresched/blob/master/tools/lib/bpf/xsk.c
struct Umem {
    fd: i32,
    umem_area: *mut libc::c_void,
}

impl Drop for Umem {
    fn drop(&mut self) {
        unsafe { libc::free(self.umem_area) };
    }
}

mod raw {
    use crate::Error;
    use libc::{c_int, c_void, off_t, size_t, socklen_t, MAP_FAILED};

    // TODO: wrap this in a stronger type that implements munmap() on Drop
    #[must_use]
    pub unsafe fn mmap(
        addr: *mut c_void,
        len: size_t,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: off_t,
    ) -> Result<*mut c_void, Error> {
        let mem = libc::mmap(addr, len, prot, flags, fd, offset);
        if mem == MAP_FAILED {
            Err(Error::Mmap(errno()))
        } else {
            Ok(mem)
        }
    }

    pub fn setsockopt(
        socket: c_int,
        level: c_int,
        name: c_int,
        value: *const c_void,
        option_len: socklen_t,
    ) -> Result<(), Error> {
        unsafe {
            if libc::setsockopt(socket, level, name, value, option_len) < 0 {
                Err(Error::SetSockOpt(errno()))
            } else {
                Ok(())
            }
        }
    }

    #[must_use]
    pub unsafe fn socket(domain: i32, ty: i32, protocol: i32) -> Result<i32, Error> {
        match libc::socket(domain, ty, protocol) {
            fd if fd > 0 => Ok(fd),
            _ => Err(Error::Socket(errno())),
        }
    }

    #[must_use]
    pub fn ptr<T>(val: T) -> *const c_void {
        std::ptr::addr_of!(val) as *const _
    }

    #[must_use]
    pub fn sizeof<T>(_val: T) -> u32 {
        std::mem::size_of::<T>() as u32
    }

    #[must_use]
    pub fn errno() -> i32 {
        unsafe { *libc::__errno_location() }
    }

    #[must_use]
    pub fn is_page_aligned(mem: *const c_void) -> bool {
        mem as u64 & (libc::_SC_PAGE_SIZE as u64 - 1) == 0
    }
}

// mod error;
// mod ring;
// mod socket;
// mod umem;
// mod util;
