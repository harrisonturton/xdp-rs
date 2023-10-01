use std::{error::Error, mem::size_of};

use xdp::{
    ring2::RingBuffer,
    sys::{
        mmap::{Behavior, Mmap, Protection, Visibility},
        socket::{Socket, XdpMmapOffsets},
    },
    umem::{Umem, UmemConfig},
};

/// count packets arriving on a given network interface
#[derive(argh::FromArgs, Debug)]
struct Args {
    /// BPF program object file
    #[argh(positional)]
    filepath: String,
    /// name of the program
    #[argh(positional)]
    program: String,
    /// network interface index
    #[argh(positional)]
    ifindex: u32,
    /// network device queue ID
    #[argh(positional)]
    queue_id: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = argh::from_env::<Args>();

    let (obj, _prog) = load_and_attach_bpf_program(&args.filepath, &args.program, args.ifindex)?;

    let socket = xdp::sys::socket::create(libc::AF_XDP, libc::SOCK_RAW, 0)?;

    let _umem = Umem::create(UmemConfig {
        socket: &socket,
        frame_count: xdp::constants::NUM_FRAMES as u32,
        frame_size: xdp::constants::FRAME_SIZE as u32,
        frame_headroom: xdp::constants::DEFAULT_FRAME_HEADROOM as u32,
    })?;

    let mut fill_queue = new_fill_ring2(&socket, xdp::constants::DEFAULT_PROD_NUM_DESCS as usize)?;

    // let map = obj.find_map("xsks_map")?;
    // let key = u32::to_be_bytes(0);
    // let value = u32::to_be_bytes(todo!());
    // map.update(&key, &value)?;

    Ok(())
}

fn load_and_attach_bpf_program(
    filepath: &str,
    program: &str,
    ifindex: u32,
) -> Result<(bpf::Object, bpf::Program), Box<dyn Error>> {
    let obj_buf = std::fs::read(filepath)?;
    let obj = bpf::Object::create(&obj_buf)?;
    obj.load()?;

    let prog = obj.find_program(program)?;
    prog.attach_xdp(ifindex)?;

    Ok((obj, prog))
}

#[must_use]
fn new_fill_ring2(socket: &Socket, size: usize) -> xdp::Result<RingBuffer<u64>> {
    socket.set_opt(libc::SOL_XDP, xdp_sys::XDP_UMEM_FILL_RING, &size)?;
    let offsets = socket.get_opt::<XdpMmapOffsets>()?;

    let len = (offsets.fr.desc + size as u64) * size_of::<u64>() as u64;
    let mmap = Mmap::builder()
        .fd(socket.fd)
        .addr(None)
        .visibility(Visibility::Shared)
        .length(len as usize)
        .offset(xdp_sys::XDP_UMEM_PGOFF_FILL_RING as i64)
        .behaviour(Behavior::PopulatePageTables)
        .protection(Protection::Read | Protection::Write)
        .build()?;

    let producer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.fr.producer as usize;
        addr as *mut u32
    };

    let consumer = {
        let addr = usize::from(mmap.addr.addr());
        let addr = addr + offsets.fr.consumer as usize;
        addr as *mut u32
    };

    let descs = unsafe {
        let addr = mmap.addr.as_ptr().offset(offsets.fr.desc as isize);
        addr as *mut u64
    };

    let buffer = RingBuffer::new(size, producer, consumer, descs);
    Ok(buffer)
}
